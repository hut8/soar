import { execSync } from 'node:child_process';
import * as path from 'node:path';

/**
 * Global setup for Playwright tests.
 *
 * Runs once before all tests to create the template database.
 * This template is used by both Rust tests and Playwright tests
 * to create isolated test databases.
 */
export default async function globalSetup() {
	console.log('\n🔧 Setting up test template database...\n');

	const scriptPath = path.resolve(__dirname, '../../scripts/setup-test-template.sh');

	try {
		execSync(scriptPath, {
			stdio: 'inherit',
			cwd: path.resolve(__dirname, '../..'),
			env: {
				...process.env,
				PGHOST: process.env.PGHOST || 'localhost',
				PGPORT: process.env.PGPORT || '5432',
				PGUSER: process.env.PGUSER || 'postgres',
				PGPASSWORD: process.env.PGPASSWORD || 'postgres'
			}
		});

		console.log('\n✅ Test template database ready\n');
	} catch (error) {
		console.error('\n❌ Failed to setup test template database\n');
		throw error;
	}
}
