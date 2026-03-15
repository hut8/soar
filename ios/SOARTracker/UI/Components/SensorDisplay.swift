import SwiftUI

struct SensorDisplay: View {
    let sensorData: SensorCollector.SensorData
    let response: TrackerFixResponse?
    let magneticHeading: Double?
    let groundPressureHpa: Double?
    let gnssAltitudeMeters: Double?
    let latitude: Double?
    let longitude: Double?
    let gpsTrack: Double?
    let verticalSpeedMps: Double?
    let onSetAltimeter: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            // Header
            Text("Sensors")
                .font(.headline)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 16)
                .padding(.top, 12)
                .padding(.bottom, 8)

            // Row 1: G-Force, Vario, Pressure
            HStack(spacing: 0) {
                SensorCell(label: "G-Force", value: gForceText)
                SensorCell(label: "Vario", value: varioText, color: varioColor)
                SensorCell(label: "Pressure", value: pressureText)
            }

            Divider().padding(.horizontal, 16)

            // Row 2: GNSS Alt, AGL, Position
            HStack(spacing: 0) {
                SensorCell(label: "GNSS Alt", value: gnssAltText)
                SensorCell(label: "AGL", value: aglText)
                SensorCell(label: "Position", value: positionText, isSmall: true)
            }

            Divider().padding(.horizontal, 16)

            // Row 3: Mag Heading, True Heading, GPS Track
            HStack(spacing: 0) {
                SensorCell(label: "Mag Hdg", value: magHeadingText)
                SensorCell(label: "True Hdg", value: trueHeadingText)
                SensorCell(label: "GPS Track", value: gpsTrackText)
            }

            Divider().padding(.horizontal, 16)

            // Altimeter section
            altimeterSection
        }
        .background(Color.cardBackground)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Computed Values

    private var gForceText: String {
        guard let x = sensorData.accelX, let y = sensorData.accelY, let z = sensorData.accelZ else {
            return "--"
        }
        let magnitude = sqrt(x * x + y * y + z * z) / 9.81
        return String(format: "%.2f g", magnitude)
    }

    private var varioFpm: Double? {
        guard let vs = verticalSpeedMps else { return nil }
        // m/s to ft/min: multiply by 196.85
        return vs * 196.85
    }

    private var varioText: String {
        guard let fpm = varioFpm else { return "--" }
        let sign = fpm >= 0 ? "+" : ""
        return "\(sign)\(Int(fpm)) fpm"
    }

    private var varioColor: Color {
        guard let fpm = varioFpm else { return .primary }
        if fpm > 50 { return .soarGreen }
        if fpm < -50 { return .soarRed }
        return .primary
    }

    private var pressureText: String {
        guard let hPa = sensorData.pressureHpa else { return "--" }
        return "\(Formatters.inHg(hPa: hPa)) inHg"
    }

    private var gnssAltText: String {
        guard let alt = gnssAltitudeMeters else { return "--" }
        return "\(Int(Formatters.metersToFeet(alt))) ft"
    }

    private var aglText: String {
        guard let agl = response?.altitudeAglFeet else { return "--" }
        return "\(Int(agl)) ft"
    }

    private var positionText: String {
        guard let lat = latitude, let lon = longitude else { return "--" }
        let latStr = String(format: "%.4f\u{00B0}%@", abs(lat), Formatters.latCardinal(lat))
        let lonStr = String(format: "%.4f\u{00B0}%@", abs(lon), Formatters.lonCardinal(lon))
        return "\(latStr) \(lonStr)"
    }

    private var magHeadingText: String {
        guard let heading = magneticHeading else { return "--" }
        return "\(Int(heading))\u{00B0}"
    }

    private var trueHeadingText: String {
        guard let mag = magneticHeading, let dec = response?.magneticDeclinationDegrees else { return "--" }
        let trueHdg = (mag + dec + 360).truncatingRemainder(dividingBy: 360)
        return "\(Int(trueHdg))\u{00B0}"
    }

    private var gpsTrackText: String {
        guard let track = gpsTrack else { return "--" }
        return "\(Int(track))\u{00B0}"
    }

    // MARK: - Altimeter Section

    @ViewBuilder
    private var altimeterSection: some View {
        VStack(spacing: 8) {
            HStack {
                Text("Altimeter")
                    .font(.subheadline.bold())

                Spacer()

                if let groundP = groundPressureHpa {
                    Text(String(format: "Ref: %.1f hPa", groundP))
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }

                Button("Set", action: onSetAltimeter)
                    .font(.caption)
                    .buttonStyle(.bordered)
                    .controlSize(.mini)
                    .disabled(sensorData.pressureHpa == nil)
            }

            if let currentP = sensorData.pressureHpa, let groundP = groundPressureHpa {
                let alt = Formatters.barometricAltitude(pressureHpa: currentP, groundPressureHpa: groundP)
                Text("\(Int(alt)) ft")
                    .font(.title2.monospacedDigit())
                    .fontWeight(.semibold)
            } else {
                Text("--")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }
}

// MARK: - Sensor Cell

private struct SensorCell: View {
    let label: String
    let value: String
    var color: Color = .primary
    var isSmall: Bool = false

    var body: some View {
        VStack(spacing: 2) {
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(isSmall ? .caption.monospacedDigit() : .callout.monospacedDigit())
                .fontWeight(.medium)
                .foregroundStyle(color)
                .lineLimit(1)
                .minimumScaleFactor(0.7)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 8)
    }
}
