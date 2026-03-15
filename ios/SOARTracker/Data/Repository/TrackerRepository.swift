import Foundation

enum TrackerRepository {
    static func submitFix(api: SoarAPI, request: TrackerFixRequest) async throws -> TrackerFixResponse {
        return try await api.submitFix(request: request)
    }
}
