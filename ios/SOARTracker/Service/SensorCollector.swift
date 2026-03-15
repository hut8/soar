import CoreMotion
import Foundation

/// Collects accelerometer and barometric pressure data from device sensors.
///
/// Accelerometer values are stored in m/s² (matching Android's SensorManager convention).
/// Barometric pressure is stored in hPa.
/// Magnetic heading is handled by TrackingService via CLLocationManager.
class SensorCollector: ObservableObject {
    struct SensorData {
        var accelX: Double? // m/s²
        var accelY: Double? // m/s²
        var accelZ: Double? // m/s²
        var pressureHpa: Double? // hPa (hectopascals)
    }

    @Published var sensorData = SensorData()

    private let motionManager = CMMotionManager()
    private let altimeter = CMAltimeter()

    func start() {
        startAccelerometer()
        startBarometer()
    }

    func stop() {
        motionManager.stopAccelerometerUpdates()
        if CMAltimeter.isRelativeAltitudeAvailable() {
            altimeter.stopRelativeAltitudeUpdates()
        }
    }

    private func startAccelerometer() {
        guard motionManager.isAccelerometerAvailable else { return }
        // ~30Hz, comparable to Android SENSOR_DELAY_GAME
        motionManager.accelerometerUpdateInterval = 1.0 / 30.0
        motionManager.startAccelerometerUpdates(to: .main) { [weak self] data, _ in
            guard let data = data else { return }
            // iOS reports acceleration in g-force; convert to m/s² for consistency with Android
            self?.sensorData.accelX = data.acceleration.x * 9.81
            self?.sensorData.accelY = data.acceleration.y * 9.81
            self?.sensorData.accelZ = data.acceleration.z * 9.81
        }
    }

    private func startBarometer() {
        guard CMAltimeter.isRelativeAltitudeAvailable() else { return }
        altimeter.startRelativeAltitudeUpdates(to: .main) { [weak self] data, _ in
            guard let data = data else { return }
            // CMAltimeter reports pressure in kPa; convert to hPa
            self?.sensorData.pressureHpa = data.pressure.doubleValue * 10.0
        }
    }
}
