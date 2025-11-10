import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Global setup for Playwright tests
 * Seeds the test database before running tests
 */
export default async function globalSetup() {
	console.log('\n🌱 Seeding test database...\n');

	try {
		// Run the seed command
		execSync(
			'DATABASE_URL=postgres://postgres:postgres@localhost:5432/soar_test ../target/release/soar seed-test-data',
			{
				cwd: __dirname,
				stdio: 'inherit',
				env: {
					...process.env,
					DATABASE_URL: 'postgres://postgres:postgres@localhost:5432/soar_test'
				}
			}
		);

		console.log('\n✅ Test database seeded successfully\n');
	} catch (error) {
		console.error('\n❌ Failed to seed test database:', error);
		throw error;
	}
}
