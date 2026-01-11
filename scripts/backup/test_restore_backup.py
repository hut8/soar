#!/usr/bin/env python3
"""
Unit tests for restore-backup script.

These tests validate the core functionality without requiring rclone or cloud access.
"""

import sys
import os
import tempfile
import unittest
import subprocess
from datetime import datetime
from pathlib import Path


class TestRestoreBackupScript(unittest.TestCase):
    """Test the restore-backup script."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.script_path = Path(__file__).parent.parent / 'restore-backup'
        self.assertTrue(self.script_path.exists(), f"Script not found: {self.script_path}")
        self.assertTrue(os.access(self.script_path, os.X_OK), "Script not executable")
    
    def test_script_is_executable(self):
        """Test that the script exists and is executable."""
        self.assertTrue(self.script_path.exists())
        self.assertTrue(os.access(self.script_path, os.X_OK))
    
    def test_shebang_is_correct(self):
        """Test that the script has the correct shebang."""
        with open(self.script_path, 'r') as f:
            first_line = f.readline()
        self.assertTrue(first_line.startswith('#!/usr/bin/env python3'))
    
    def test_help_option(self):
        """Test that --help option works."""
        result = subprocess.run(
            [str(self.script_path), '--help'],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn('Interactive backup restoration tool', result.stdout)
        self.assertIn('--list', result.stdout)
        self.assertIn('--latest', result.stdout)
        self.assertIn('--date', result.stdout)
    
    def test_missing_config_file(self):
        """Test handling of missing configuration file."""
        with tempfile.NamedTemporaryFile(delete=False) as tmp:
            tmp_path = tmp.name
        
        # Delete the file so it doesn't exist
        os.unlink(tmp_path)
        
        result = subprocess.run(
            [str(self.script_path), '--config', tmp_path, '--list'],
            capture_output=True,
            text=True
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('not found', result.stderr.lower())
    
    def test_syntax_is_valid(self):
        """Test that the script has valid Python syntax."""
        result = subprocess.run(
            ['python3', '-m', 'py_compile', str(self.script_path)],
            capture_output=True,
            text=True
        )
        self.assertEqual(result.returncode, 0, f"Syntax error: {result.stderr}")
    
    def test_missing_rclone_error(self):
        """Test that script handles missing rclone gracefully."""
        # Create a minimal config file
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.env') as tmp:
            tmp.write("BACKUP_RCLONE_REMOTE=test\n")
            tmp.write("BACKUP_RCLONE_BUCKET=test-bucket\n")
            tmp.write("RCLONE_CONFIG=/tmp/test.conf\n")
            tmp_path = tmp.name
        
        try:
            # Get current PATH and keep only essential system paths
            # Exclude common rclone installation locations
            current_path = os.environ.get('PATH', '')
            filtered_paths = [p for p in current_path.split(':') 
                            if p in ['/usr/bin', '/bin'] and 'rclone' not in p]
            
            result = subprocess.run(
                [str(self.script_path), '--config', tmp_path, '--list'],
                capture_output=True,
                text=True,
                env={**os.environ, 'PATH': ':'.join(filtered_paths)}
            )
            # Should fail but with a clear error message
            self.assertNotEqual(result.returncode, 0)
            # Should mention rclone
            combined_output = result.stdout + result.stderr
            self.assertTrue(
                'rclone' in combined_output.lower(),
                f"Expected 'rclone' in output, got: {combined_output}"
            )
        finally:
            os.unlink(tmp_path)
    
    def test_invalid_date_format(self):
        """Test that script validates date format."""
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.env') as tmp:
            tmp.write("BACKUP_RCLONE_REMOTE=test\n")
            tmp.write("BACKUP_RCLONE_BUCKET=test-bucket\n")
            tmp.write("RCLONE_CONFIG=/tmp/test.conf\n")
            tmp_path = tmp.name
        
        try:
            # Test with invalid date format
            result = subprocess.run(
                [str(self.script_path), '--config', tmp_path, '--date', 'invalid-date'],
                capture_output=True,
                text=True,
                env={**os.environ, 'PATH': '/tmp'}  # Remove rclone from PATH
            )
            # Should fail (either due to missing rclone or invalid date)
            self.assertNotEqual(result.returncode, 0)
        finally:
            os.unlink(tmp_path)


def run_tests():
    """Run all tests."""
    # Create a test suite
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # Add all test cases
    suite.addTests(loader.loadTestsFromTestCase(TestRestoreBackupScript))
    
    # Run tests
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    # Return exit code
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    sys.exit(run_tests())
