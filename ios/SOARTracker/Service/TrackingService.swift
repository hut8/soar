import CoreLocation
import Foundation
import UIKit

/// Central service that coordinates GPS tracking, sensor collection, and fix submission.
///
/// Mirrors the Android TrackingService: on each location update, collects sensor data
/// and submits a TrackerFixRequest to the backend.
class TrackingService: NSObject, ObservableObject {
    @Published var isTracking = false
    @Published var latestResponse: TrackerFixResponse?
    @Published var lastError: String?
    @Published var lastUpdateTime: Date?
    @Published var magneticHeading: Double?
    @Published var groundPressureHpa: Double?

    // GPS-derived values for sensor display
    @Published var lastLatitude: Double?
    @Published var lastLongitude: Double?
    @Published var lastGNSSAltitudeMeters: Double?
    @Published var lastGPSTrack: Double?
    @Published var lastVerticalSpeedMps: Double?

    let sensorCollector = SensorCollector()

    private var locationManager: CLLocationManager?
    private var api: SoarAPI?
    private var altitudeHistory: [(TimeInterval, Double)] = []
    private var currentTask: Task<Void, Never>?

    private let altitudeHistorySize = 5

    // MARK: - Location Authorization

    var authorizationStatus: CLAuthorizationStatus {
        locationManager?.authorizationStatus ?? .notDetermined
    }

    func requestLocationPermission() {
        let lm = CLLocationManager()
        lm.delegate = self
        locationManager = lm
        lm.requestWhenInUseAuthorization()
    }

    // MARK: - Tracking Control

    func startTracking(api: SoarAPI) {
        self.api = api
        isTracking = true
        lastError = nil
        altitudeHistory = []

        // Prevent screen dimming during flight
        UIApplication.shared.isIdleTimerDisabled = true

        // Start sensor collection
        sensorCollector.start()

        // Configure and start location updates
        let lm = locationManager ?? CLLocationManager()
        lm.delegate = self
        lm.desiredAccuracy = kCLLocationAccuracyBest
        lm.distanceFilter = kCLDistanceFilterNone
        lm.allowsBackgroundLocationUpdates = true
        lm.pausesLocationUpdatesAutomatically = false
        lm.showsBackgroundLocationIndicator = true
        lm.activityType = .airborne
        lm.startUpdatingLocation()
        lm.startUpdatingHeading()
        locationManager = lm
    }

    func stopTracking() {
        isTracking = false
        currentTask?.cancel()

        locationManager?.stopUpdatingLocation()
        locationManager?.stopUpdatingHeading()

        sensorCollector.stop()

        UIApplication.shared.isIdleTimerDisabled = false
    }

    // MARK: - Fix Submission

    private func submitFix(location: CLLocation) {
        guard let api = api else { return }

        // Update GPS-derived published properties
        lastLatitude = location.coordinate.latitude
        lastLongitude = location.coordinate.longitude
        lastGNSSAltitudeMeters = location.altitude
        if location.course >= 0 {
            lastGPSTrack = location.course
        }

        let sensor = sensorCollector.sensorData
        let verticalSpeed = calculateVerticalSpeed(altitude: location.altitude)
        lastVerticalSpeedMps = verticalSpeed

        let request = TrackerFixRequest(
            latitude: location.coordinate.latitude,
            longitude: location.coordinate.longitude,
            heading: magneticHeading,
            altitudeMeters: location.altitude,
            speedMps: location.speed >= 0 ? location.speed : nil,
            accuracyMeters: location.horizontalAccuracy >= 0 ? location.horizontalAccuracy : nil,
            pressureHpa: sensor.pressureHpa,
            // Convert m/s² back to g-force for API submission (matching Android behavior)
            accelX: sensor.accelX.map { $0 / 9.81 },
            accelY: sensor.accelY.map { $0 / 9.81 },
            accelZ: sensor.accelZ.map { $0 / 9.81 },
            verticalSpeedMps: verticalSpeed
        )

        // Cancel any in-flight request to avoid backlog
        currentTask?.cancel()

        currentTask = Task {
            do {
                let response = try await TrackerRepository.submitFix(api: api, request: request)
                guard !Task.isCancelled else { return }
                await MainActor.run {
                    self.latestResponse = response
                    self.lastError = nil
                    self.lastUpdateTime = Date()
                    self.checkAutoAltimeter(response: response)
                }
            } catch {
                guard !Task.isCancelled else { return }
                await MainActor.run {
                    self.lastError = error.localizedDescription
                }
            }
        }
    }

    // MARK: - Calculations

    /// Rolling vertical speed from GPS altitude history (m/s), matching Android's 5-sample window.
    private func calculateVerticalSpeed(altitude: Double) -> Double? {
        let now = ProcessInfo.processInfo.systemUptime
        altitudeHistory.append((now, altitude))
        while altitudeHistory.count > altitudeHistorySize {
            altitudeHistory.removeFirst()
        }
        guard altitudeHistory.count >= 2 else { return nil }
        let first = altitudeHistory.first!
        let last = altitudeHistory.last!
        let dt = last.0 - first.0
        guard dt >= 0.5 else { return nil }
        return (last.1 - first.1) / dt
    }

    /// Automatically set ground pressure when aircraft is near ground level.
    private func checkAutoAltimeter(response: TrackerFixResponse) {
        guard groundPressureHpa == nil else { return }
        guard let agl = response.altitudeAglFeet, agl >= -50 && agl <= 50 else { return }
        guard let pressure = sensorCollector.sensorData.pressureHpa else { return }
        groundPressureHpa = pressure
    }
}

// MARK: - CLLocationManagerDelegate

extension TrackingService: CLLocationManagerDelegate {
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard isTracking, let location = locations.last else { return }
        submitFix(location: location)
    }

    func locationManager(_ manager: CLLocationManager, didUpdateHeading newHeading: CLHeading) {
        if newHeading.magneticHeading >= 0 {
            magneticHeading = newHeading.magneticHeading
        }
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        lastError = error.localizedDescription
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        // After getting "When In Use", request "Always" for background tracking
        if manager.authorizationStatus == .authorizedWhenInUse {
            manager.requestAlwaysAuthorization()
        }
        objectWillChange.send()
    }
}
