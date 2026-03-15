import SwiftUI

struct LoginScreen: View {
    @EnvironmentObject var authState: ObservableAuthState

    @State private var email = ""
    @State private var password = ""
    @State private var selectedServer: Server = .from(url: TokenManager.shared.serverURL)
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var successMessage: String?

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 24) {
                    // Logo / Title
                    VStack(spacing: 8) {
                        if isLoading {
                            RadarSweep(size: 60)
                                .padding(.bottom, 8)
                        } else {
                            Image(systemName: "airplane")
                                .font(.system(size: 48))
                                .foregroundStyle(.tint)
                                .padding(.bottom, 8)
                        }

                        Text("SOAR Tracker")
                            .font(.largeTitle.bold())

                        Text("Flight Telemetry")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.top, 40)

                    // Error / Success messages
                    if let error = errorMessage {
                        MessageBanner(message: error, isError: true)
                    }
                    if let success = successMessage {
                        MessageBanner(message: success, isError: false)
                    }

                    // Login form
                    VStack(spacing: 16) {
                        TextField("Email", text: $email)
                            .textContentType(.emailAddress)
                            .keyboardType(.emailAddress)
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                            .textFieldStyle(.roundedBorder)

                        SecureField("Password", text: $password)
                            .textContentType(.password)
                            .textFieldStyle(.roundedBorder)

                        Button(action: login) {
                            Text("Log In")
                                .frame(maxWidth: .infinity)
                                .fontWeight(.semibold)
                        }
                        .buttonStyle(.borderedProminent)
                        .controlSize(.large)
                        .disabled(email.isEmpty || password.isEmpty || isLoading)
                    }

                    // Forgot password
                    Button(action: forgotPassword) {
                        Text("Forgot password?")
                            .font(.footnote)
                    }

                    // Server selection
                    Divider()

                    VStack(spacing: 8) {
                        Text("Server")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        Picker("Server", selection: $selectedServer) {
                            ForEach(Server.allCases, id: \.self) { server in
                                Text(server.label).tag(server)
                            }
                        }
                        .pickerStyle(.segmented)
                    }
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 40)
            }
            .navigationBarHidden(true)
        }
    }

    private func login() {
        errorMessage = nil
        successMessage = nil
        isLoading = true

        Task {
            do {
                try await authState.login(
                    serverURL: selectedServer.url,
                    email: email,
                    password: password
                )
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                }
            }
            await MainActor.run {
                isLoading = false
            }
        }
    }

    private func forgotPassword() {
        guard !email.isEmpty else {
            // Open password reset page in browser
            if let url = URL(string: "\(selectedServer.url)/forgot-password") {
                UIApplication.shared.open(url)
            }
            return
        }

        errorMessage = nil
        successMessage = nil
        isLoading = true

        Task {
            do {
                try await authState.requestPasswordReset(
                    serverURL: selectedServer.url,
                    email: email
                )
                await MainActor.run {
                    successMessage = "Password reset email sent to \(email)"
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                }
            }
            await MainActor.run {
                isLoading = false
            }
        }
    }
}

// MARK: - Message Banner

private struct MessageBanner: View {
    let message: String
    let isError: Bool

    var body: some View {
        HStack {
            Image(systemName: isError ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
                .foregroundStyle(isError ? .red : .green)
            Text(message)
                .font(.footnote)
                .foregroundStyle(isError ? .red : .green)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(isError ? Color.red.opacity(0.1) : Color.green.opacity(0.1))
        )
    }
}
