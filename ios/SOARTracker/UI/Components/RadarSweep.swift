import SwiftUI

/// Animated radar loading indicator matching the frontend RadarLoader and Android RadarSweep.
/// Displays concentric rings with a rotating sweep line.
struct RadarSweep: View {
    var size: CGFloat = 20

    @State private var rotation: Double = 0

    var body: some View {
        ZStack {
            // Concentric rings
            ForEach([0.9, 0.6, 0.3], id: \.self) { scale in
                Circle()
                    .stroke(Color.accentColor.opacity(0.3), lineWidth: 1)
                    .frame(width: size * scale, height: size * scale)
            }

            // Center dot
            Circle()
                .fill(Color.accentColor)
                .frame(width: size * 0.08, height: size * 0.08)

            // Sweep line
            SweepLine()
                .stroke(Color.accentColor.opacity(0.6), lineWidth: 1.5)
                .frame(width: size, height: size)
                .rotationEffect(.degrees(rotation))
        }
        .frame(width: size, height: size)
        .onAppear {
            withAnimation(.linear(duration: 2).repeatForever(autoreverses: false)) {
                rotation = 360
            }
        }
    }
}

// MARK: - Sweep Line Shape

private struct SweepLine: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let radius = min(rect.width, rect.height) / 2 * 0.9
        path.move(to: center)
        path.addLine(to: CGPoint(x: center.x, y: center.y - radius))
        return path
    }
}
