#!/usr/bin/env python3
"""
Database Reset Script
=====================

This script will PERMANENTLY DROP and RECREATE the entire database.
It forcibly disconnects all clients, drops the database, and recreates it fresh.

WARNING: This operation is IRREVERSIBLE. Use with extreme caution.
"""

import sys
import os
import psycopg
from typing import List
import argparse


def get_database_name(environment: str) -> str:
    """Get the database name for the specified environment."""
    if environment == "dev":
        return "soar_dev"
    elif environment == "production":
        return "soar"
    else:
        raise ValueError(f"Invalid environment: {environment}")


def get_connection_params() -> dict:
    """Get database connection parameters from environment variables."""
    return {
        'host': os.getenv('DB_HOST', 'localhost'),
        'port': os.getenv('DB_PORT', '5432'),
        'user': os.getenv('DB_USER', 'postgres'),
        'password': os.getenv('DB_PASSWORD', ''),
    }


def get_admin_database_url() -> str:
    """Get the database URL for connecting to the admin database (postgres)."""
    params = get_connection_params()
    if params['password']:
        return f"postgresql://{params['user']}:{params['password']}@{params['host']}:{params['port']}/postgres"
    else:
        return f"postgresql://{params['user']}@{params['host']}:{params['port']}/postgres"


def confirm_operation(environment: str, db_name: str) -> bool:
    """Get user confirmation for the destructive operation."""
    print(f"\n{'='*60}")
    print(f"🚨 DANGER: DATABASE RESET OPERATION 🚨")
    print(f"{'='*60}")
    print(f"Environment: {environment.upper()}")
    print(f"Database: {db_name}")
    print(f"Operation: DROP and RECREATE database")

    print(f"\n{'='*60}")
    print("⚠️  WARNING: This will PERMANENTLY DROP THE ENTIRE DATABASE!")
    print("⚠️  All data, tables, indexes, and schema will be LOST!")
    print("⚠️  This operation is IRREVERSIBLE!")
    print("⚠️  Make sure you have backups if needed!")
    print(f"{'='*60}")

    # Multiple confirmation steps for safety
    confirm1 = input(f"\nType 'DROP DATABASE' to proceed: ")
    if confirm1 != "DROP DATABASE":
        print("❌ Operation cancelled.")
        return False

    confirm2 = input(f"Type the database name '{db_name}' to confirm: ")
    if confirm2 != db_name:
        print("❌ Database name mismatch. Operation cancelled.")
        return False

    confirm3 = input(f"Type the environment name '{environment}' to confirm: ")
    if confirm3 != environment:
        print("❌ Environment mismatch. Operation cancelled.")
        return False

    if environment == "production":
        confirm4 = input("⚠️  FINAL WARNING: You are about to DROP PRODUCTION DATABASE!\nType 'PRODUCTION DROP CONFIRMED' to proceed: ")
        if confirm4 != "PRODUCTION DROP CONFIRMED":
            print("❌ Final confirmation failed. Operation cancelled.")
            return False

    return True


def reset_database(environment: str, dry_run: bool = False) -> None:
    """Drop and recreate the specified database."""
    db_name = get_database_name(environment)

    try:
        # Connect to the postgres admin database
        admin_db_url = get_admin_database_url()
        print(f"Connecting to postgres admin database...")

        conn = psycopg.connect(admin_db_url, autocommit=True)
        cursor = conn.cursor()

        # Check if database exists
        print(f"Checking if database '{db_name}' exists...")
        cursor.execute("SELECT 1 FROM pg_database WHERE datname = %s", (db_name,))
        db_exists = cursor.fetchone() is not None

        if not db_exists:
            print(f"Database '{db_name}' does not exist.")
            if dry_run:
                print(f"🔍 DRY RUN: Would create database '{db_name}'")
                return
            else:
                create_db = input(f"Create database '{db_name}'? (y/N): ")
                if create_db.lower() != 'y':
                    print("❌ Operation cancelled.")
                    return
        else:
            # Get user confirmation for destructive operation
            if not dry_run and not confirm_operation(environment, db_name):
                return

        if dry_run:
            print(f"\n🔍 DRY RUN MODE - No actual changes will be made")
            if db_exists:
                print(f"Would execute the following operations:")
                print(f"  1. Terminate all connections to '{db_name}'")
                print(f"  2. DROP DATABASE {db_name};")
                print(f"  3. CREATE DATABASE {db_name};")
            else:
                print(f"  1. CREATE DATABASE {db_name};")
            return

        if db_exists:
            # Terminate all connections to the target database
            print(f"\n🔌 Terminating all connections to '{db_name}'...")
            cursor.execute("""
                SELECT pg_terminate_backend(pid)
                FROM pg_stat_activity
                WHERE datname = %s AND pid <> pg_backend_pid()
            """, (db_name,))
            terminated_count = cursor.rowcount
            print(f"    ✅ Terminated {terminated_count} connections")

            # Drop the database
            print(f"\n🗑️  Dropping database '{db_name}'...")
            cursor.execute(f'DROP DATABASE "{db_name}"')
            print(f"    ✅ Database '{db_name}' dropped successfully")

        # Create the database
        print(f"\n🏗️  Creating database '{db_name}'...")
        cursor.execute(f'CREATE DATABASE "{db_name}"')
        print(f"    ✅ Database '{db_name}' created successfully")

        print(f"\n✅ Database reset completed for {environment} environment!")
        print(f"💡 Remember to run migrations to set up the schema:")
        print(f"   diesel migration run --database-url postgresql://user:pass@host:port/{db_name}")

    except psycopg.Error as e:
        print(f"❌ Database error: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Error: {e}")
        sys.exit(1)
    finally:
        if 'cursor' in locals():
            cursor.close()
        if 'conn' in locals():
            conn.close()


def main():
    parser = argparse.ArgumentParser(
        description="Reset database by dropping and recreating it",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s dev                    # Reset dev database
  %(prog)s production             # Reset production database
  %(prog)s dev --dry-run          # Show what would be done (safe)

WARNING: This will permanently drop and recreate the entire database!
        """
    )

    parser.add_argument(
        'environment',
        choices=['dev', 'production'],
        help='Database environment to reset'
    )

    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Show what would be done without making changes'
    )

    args = parser.parse_args()

    # Show banner
    print("Database Reset Script")
    print("=" * 21)

    if args.dry_run:
        print("🔍 Running in DRY RUN mode - no changes will be made")

    reset_database(args.environment, args.dry_run)


if __name__ == "__main__":
    main()