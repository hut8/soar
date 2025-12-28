import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { existsSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '..');

/**
 * Global setup for Playwright tests
 * Builds the application and seeds the test database
 */
export default async function globalSetup() {
	// Check if release binary exists with web assets
	const releaseBinary = join(projectRoot, 'target/release/soar');
	const webBuildDir = join(__dirname, 'build');

	if (!existsSync(releaseBinary) || !existsSync(webBuildDir)) {
		console.log('\n🔨 Building application with embedded web assets...\n');

		try {
			// Build web assets
			console.log('Building web assets...');
			execSync('npm run build', {
				cwd: __dirname,
				stdio: 'inherit'
			});

			// Build Rust release binary with embedded web assets
			console.log('\nBuilding Rust release binary...');
			execSync('cargo build --release', {
				cwd: projectRoot,
				stdio: 'inherit'
			});

			console.log('\n✅ Application built successfully\n');
		} catch (error) {
			console.error('\n❌ Failed to build application:', error);
			throw error;
		}
	} else {
		console.log('\n✅ Using existing release binary with embedded assets\n');
	}

	console.log('🗂️  Setting up test template database...\n');

	try {
		// Create the template database (with migrations)
		console.log('Creating template database...');
		execSync('../scripts/setup-test-template.sh', {
			cwd: __dirname,
			stdio: 'inherit',
			env: {
				...process.env,
				PGHOST: 'localhost',
				PGPORT: '5432',
				PGUSER: 'postgres',
				PGPASSWORD: 'postgres'
			}
		});

		// Seed the template database with test data
		console.log('\nSeeding template database...');
		execSync('../target/release/soar seed-test-data', {
			cwd: __dirname,
			stdio: 'inherit',
			env: {
				...process.env,
				DATABASE_URL: 'postgres://postgres:postgres@localhost:5432/soar_test_template'
			}
		});

		console.log('\n✅ Test template database ready\n');
		console.log('ℹ️  Each test worker will create its own database from this template\n');
	} catch (error) {
		console.error('\n❌ Failed to setup test template database:', error);
		throw error;
	}
}
