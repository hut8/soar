/**
 * Test data fixtures
 *
 * Provides mock/test data for use in E2E tests
 */

/**
 * Test user credentials
 */
export const testUsers = {
	validUser: {
		email: 'test@example.com',
		password: 'testpassword123'
	},
	invalidUser: {
		email: 'invalid@example.com',
		password: 'wrongpassword'
	},
	newUser: {
		email: 'newuser@example.com',
		password: 'newpassword123',
		name: 'New Test User'
	}
};

/**
 * Test device data
 */
export const testDevices = {
	validRegistration: 'N12345',
	validDeviceAddress: {
		type: 'I', // ICAO
		address: 'ABC123'
	},
	invalidRegistration: 'INVALIDREG999999'
};

/**
 * Test club data
 */
export const testClubs = {
	validClubId: '1',
	validClubName: 'Test Soaring Club'
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
