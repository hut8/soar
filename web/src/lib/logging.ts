/**
 * LogTape configuration for the frontend.
 *
 * Usage:
 *   import { getLogger } from '$lib/logging';
 *   const logger = getLogger(['myModule']);
 *   logger.info('Hello, world!');
 *   logger.error('An error occurred', { error });
 */

import { configure, getConsoleSink, getLogger as getLogtapeLogger } from '@logtape/logtape';
import type { Logger } from '@logtape/logtape';
import { browser } from '$app/environment';

let configured = false;

/**
 * Configure LogTape. Should be called once at app startup.
 * Safe to call multiple times - only configures once.
 */
export async function configureLogging(): Promise<void> {
	if (configured || !browser) return;

	await configure({
		sinks: {
			console: getConsoleSink()
		},
		loggers: [
			{
				category: ['soar'],
				lowestLevel: 'debug',
				sinks: ['console']
			}
		]
	});

	configured = true;
}

/**
 * Get a logger for a specific category.
 *
 * @param category - The logger category (e.g., ['soar', 'FixFeed'])
 * @returns A LogTape Logger instance
 *
 * @example
 * const logger = getLogger(['soar', 'FixFeed']);
 * logger.info('WebSocket connected');
 * logger.warn('Connection lost', { code: 1006 });
 * logger.error('Failed to parse message', { error });
 */
export function getLogger(category: string[]): Logger {
	return getLogtapeLogger(category);
}

// Re-export Logger type for convenience
export type { Logger };
