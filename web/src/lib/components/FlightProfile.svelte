<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import Plotly from 'plotly.js-dist-min';
	import { theme } from '$lib/stores/theme';
	import type { Fix } from '$lib/types';

	let {
		fixes,
		hasAglData = false,
		onHover = undefined,
		onUnhover = undefined,
		onClick = undefined,
		recreateTrigger = $bindable(0)
	}: {
		fixes: Fix[];
		hasAglData?: boolean;
		onHover?: (fix: Fix) => void;
		onUnhover?: () => void;
		onClick?: (fix: Fix) => void;
		recreateTrigger?: number;
	} = $props();

	let chartContainer = $state<HTMLElement>();
	let chartInitialized = $state(false);

	// Store event listener functions so we can remove them
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let hoverListener: ((event: any) => void) | null = null;
	let unhoverListener: (() => void) | null = null;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let clickListener: ((event: any) => void) | null = null;

	// Get Plotly layout configuration based on theme
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function getPlotlyLayout(isDark: boolean): any {
		return {
			xaxis: {
				type: 'date',
				tickformat: '%H:%M',
				color: isDark ? '#9ca3af' : '#6b7280',
				gridcolor: isDark ? '#374151' : '#e5e7eb'
			},
			yaxis: {
				rangemode: 'tozero',
				color: isDark ? '#9ca3af' : '#6b7280',
				gridcolor: isDark ? '#374151' : '#e5e7eb'
			},
			yaxis2: {
				overlaying: 'y',
				side: 'right',
				rangemode: 'tozero',
				color: isDark ? '#9ca3af' : '#6b7280',
				showgrid: false
			},
			hovermode: 'x unified',
			showlegend: true,
			legend: {
				x: 0.5,
				y: -0.25,
				xanchor: 'center',
				yanchor: 'top',
				orientation: 'h',
				bgcolor: isDark ? 'rgba(31, 41, 55, 0.8)' : 'rgba(255, 255, 255, 0.8)',
				font: {
					color: isDark ? '#e5e7eb' : '#111827'
				}
			},
			margin: { l: 40, r: 40, t: 40, b: 100 },
			paper_bgcolor: isDark ? '#1f2937' : '#ffffff',
			plot_bgcolor: isDark ? '#111827' : '#f9fafb'
		};
	}

	async function createChart() {
		if (!chartContainer || fixes.length === 0) return;

		try {
			// Remove old event listeners if they exist
			if (hoverListener && chartContainer) {
				chartContainer.removeEventListener('plotly_hover', hoverListener);
			}
			if (unhoverListener && chartContainer) {
				chartContainer.removeEventListener('plotly_unhover', unhoverListener);
			}
			if (clickListener && chartContainer) {
				chartContainer.removeEventListener('plotly_click', clickListener);
			}

			const fixesInOrder = [...fixes].reverse();

			// Define large gap threshold (5 minutes in milliseconds)
			const LARGE_GAP_THRESHOLD_MS = 5 * 60 * 1000;

			// Process fixes to insert nulls for large time gaps and null values
			const timestamps: (Date | null)[] = [];
			const altitudesMsl: (number | null)[] = [];
			const altitudesAgl: (number | null)[] = [];
			const groundSpeeds: (number | null)[] = [];

			for (let i = 0; i < fixesInOrder.length; i++) {
				const fix = fixesInOrder[i];
				const timestamp = new Date(fix.timestamp);

				// Check for large time gap (except for first fix)
				if (i > 0) {
					const prevTimestamp = new Date(fixesInOrder[i - 1].timestamp);
					const timeDiff = timestamp.getTime() - prevTimestamp.getTime();

					if (timeDiff > LARGE_GAP_THRESHOLD_MS) {
						// Insert null point to create gap in the line
						timestamps.push(null);
						altitudesMsl.push(null);
						altitudesAgl.push(null);
						groundSpeeds.push(null);
					}
				}

				// Add current fix data - use null instead of 0 for missing values
				timestamps.push(timestamp);
				altitudesMsl.push(fix.altitudeMslFeet ?? null);
				altitudesAgl.push(fix.altitudeAglFeet ?? null);
				groundSpeeds.push(fix.groundSpeedKnots ?? null);
			}

			const traces = [
				{
					x: timestamps,
					y: altitudesMsl,
					type: 'scatter' as const,
					mode: 'lines' as const,
					name: 'MSL Altitude (ft)',
					line: { color: '#3b82f6', width: 2 },
					connectgaps: false,
					hovertemplate: '<b>MSL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
				}
			];

			if (hasAglData) {
				traces.push({
					x: timestamps,
					y: altitudesAgl,
					type: 'scatter' as const,
					mode: 'lines' as const,
					name: 'AGL Altitude (ft)',
					line: { color: '#10b981', width: 2 },
					connectgaps: false,
					hovertemplate: '<b>AGL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
				});
			}

			// Add ground speed trace on secondary y-axis
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const groundSpeedTrace: any = {
				x: timestamps,
				y: groundSpeeds,
				type: 'scatter' as const,
				mode: 'lines' as const,
				name: 'Ground Speed (kt, Right Y Axis)',
				line: { color: '#f59e0b', width: 2 },
				connectgaps: false,
				yaxis: 'y2',
				hovertemplate: '<b>GS:</b> %{y:.0f} kt<br>%{x}<extra></extra>'
			};
			traces.push(groundSpeedTrace);

			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const layout: any = getPlotlyLayout($theme === 'dark');
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const config: any = {
				responsive: true,
				displayModeBar: true,
				displaylogo: false,
				modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d', 'zoom2d', 'zoomIn2d', 'zoomOut2d'],
				scrollZoom: false
			};

			await Plotly.newPlot(chartContainer, traces, layout, config);
			chartInitialized = true;

			// Add hover event handlers if callbacks are provided
			if (onHover) {
				// Use Plotly's event system
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(chartContainer as any).on('plotly_hover', (data: any) => {
					if (data.points && data.points.length > 0) {
						const pointIndex = data.points[0].pointIndex;
						if (pointIndex >= 0 && pointIndex < fixesInOrder.length) {
							const fix = fixesInOrder[pointIndex];
							onHover(fix);
						}
					}
				});

				// Also use addEventListener as backup
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				hoverListener = (event: any) => {
					const data_event = event.detail || event;
					if (data_event.points && data_event.points.length > 0) {
						const pointIndex = data_event.points[0].pointIndex;
						if (pointIndex >= 0 && pointIndex < fixesInOrder.length) {
							const fix = fixesInOrder[pointIndex];
							onHover(fix);
						}
					}
				};
				chartContainer.addEventListener('plotly_hover', hoverListener);
			}

			if (onUnhover) {
				// Use Plotly's event system
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(chartContainer as any).on('plotly_unhover', () => {
					onUnhover();
				});

				unhoverListener = () => {
					onUnhover();
				};
				chartContainer.addEventListener('plotly_unhover', unhoverListener);
			}

			// Add click event handler if callback is provided
			if (onClick) {
				// Use Plotly's event system
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(chartContainer as any).on('plotly_click', (data: any) => {
					if (data.points && data.points.length > 0) {
						const pointIndex = data.points[0].pointIndex;
						if (pointIndex >= 0 && pointIndex < fixesInOrder.length) {
							const fix = fixesInOrder[pointIndex];
							onClick(fix);
						}
					}
				});

				// Also use addEventListener as backup
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				clickListener = (event: any) => {
					const data_event = event.detail || event;
					if (data_event.points && data_event.points.length > 0) {
						const pointIndex = data_event.points[0].pointIndex;
						if (pointIndex >= 0 && pointIndex < fixesInOrder.length) {
							const fix = fixesInOrder[pointIndex];
							onClick(fix);
						}
					}
				};
				chartContainer.addEventListener('plotly_click', clickListener);
			}
		} catch (error) {
			console.error('Failed to create flight profile chart:', error);
		}
	}

	// Initialize chart on mount
	onMount(() => {
		createChart();
	});

	// Update chart when theme changes
	$effect(() => {
		const currentTheme = $theme;

		if (chartInitialized && chartContainer && Plotly && fixes.length > 0) {
			const isDark = currentTheme === 'dark';
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const currentData = (chartContainer as any).data || [];

			if (currentData.length > 0) {
				Plotly.react(chartContainer, currentData, getPlotlyLayout(isDark));
			}
		}
	});

	// Recreate chart when trigger changes (for collapsible panels, etc.)
	$effect(() => {
		// Watch recreateTrigger - when it changes, recreate the chart
		if (recreateTrigger > 0 && chartContainer && fixes.length > 0) {
			// Wait for next tick to ensure DOM is ready
			setTimeout(() => {
				createChart();
			}, 100);
		}
	});

	// Cleanup
	onDestroy(() => {
		// Remove event listeners
		if (hoverListener && chartContainer) {
			chartContainer.removeEventListener('plotly_hover', hoverListener);
		}
		if (unhoverListener && chartContainer) {
			chartContainer.removeEventListener('plotly_unhover', unhoverListener);
		}
		if (clickListener && chartContainer) {
			chartContainer.removeEventListener('plotly_click', clickListener);
		}

		// Purge Plotly chart
		if (chartContainer) {
			Plotly.purge(chartContainer);
		}
	});
</script>

<div class="relative h-full w-full">
	<div bind:this={chartContainer} class="h-full w-full"></div>
</div>
