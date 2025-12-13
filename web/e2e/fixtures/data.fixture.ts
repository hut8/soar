/**
 * Test data fixtures
 *
 * Provides mock/test data for use in E2E tests.
 * Test data is seeded into the soar_test database by the setup-test-db.sh script.
 *
 * Environment variables (set by CI or local .env.test):
 * - TEST_USER_EMAIL: Test user email (default: test@example.com)
 * - TEST_USER_PASSWORD: Test user password (default: testpassword123)
 */

/**
 * Test user credentials
 *
 * These correspond to users created by the seed-test-data command.
 * The validUser is created with known credentials from environment variables.
 */
export const testUsers = {
	validUser: {
		email: process.env.TEST_USER_EMAIL || 'test@example.com',
		password: process.env.TEST_USER_PASSWORD || 'testpassword123'
	},
	invalidUser: {
		email: 'invalid@example.com',
		password: 'wrongpassword'
	},
	newUser: {
		email: `newuser-${Date.now()}@example.com`, // Unique email for registration tests
		password: 'newpassword123',
		name: 'New Test User'
	}
};

/**
 * Test device data
 *
 * These correspond to devices created by the seed-test-data command.
 * Known test devices: N12345, N54321, N98765
 */
export const testAircraft = {
	validRegistration: 'N12345', // Known test device from seed data
	validAircraftAddress: {
		type: 'I', // ICAO
		address: 'ABC123' // Corresponds to N12345
	},
	invalidRegistration: 'INVALIDREG999999' // Should not exist in test data
};

/**
 * Test club data
 *
 * The primary test club is created by the seed-test-data command with a deterministic UUID.
 */
export const testClubs = {
	validClubName: 'Test Soaring Club', // Created by seed data
	validClubId: '00000000-0000-0000-0000-000000000001' // Deterministic UUID from seed data
};

/**
 * Wait times for various operations (in milliseconds)
 */
export const waitTimes = {
	short: 1000, // 1 second
	medium: 3000, // 3 seconds
	long: 5000, // 5 seconds
	apiCall: 10000 // 10 seconds for API calls
};
