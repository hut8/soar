#!/usr/bin/env node

/**
 * Split the soar run dashboard into focused sub-dashboards
 */

const fs = require('fs');
const path = require('path');

const sourcePath = './infrastructure/grafana-dashboard-run.json';
const outputDir = './infrastructure';

// Read source dashboard
const source = JSON.parse(fs.readFileSync(sourcePath, 'utf8'));

// Define the splits with panel ranges
const splits = {
  'grafana-dashboard-run-core.json': {
    title: 'SOAR Run - Core System & Infrastructure',
    description: 'System-level metrics, database, and message publishing',
    uid: 'soar-run-core',
    panelRanges: [
      { start: 0, end: 4 },    // Process Metrics (row + 4 panels)
      { start: 59, end: 61 },  // NATS Publisher (row + 2 panels)
      { start: 56, end: 58 },  // Aircraft Processing Latency Breakdown (row + 2 panels)
      { start: 72, end: 74 }   // Database Connection Pool (row + 2 panels)
    ]
  },
  'grafana-dashboard-run-ingestion.json': {
    title: 'SOAR Run - Data Ingestion Pipelines',
    description: 'OGN/APRS and ADS-B Beast message intake',
    uid: 'soar-run-ingestion',
    panelRanges: [
      { start: 30, end: 33 },  // OGN Processing (row + 3 panels)
      { start: 34, end: 40 }   // Beast (ADS-B) Processing (row + 6 panels)
    ]
  },
  'grafana-dashboard-run-routing.json': {
    title: 'SOAR Run - Packet Processing & Routing',
    description: 'Core packet routing, parsing, and distribution',
    uid: 'soar-run-routing',
    panelRanges: [
      { start: 12, end: 16 },  // APRS Router & Parser (row + 4 panels)
      { start: 17, end: 29 }   // Packet Processing (row + 12 panels)
    ]
  },
  'grafana-dashboard-run-flights.json': {
    title: 'SOAR Run - Aircraft & Flight Tracking',
    description: 'Flight lifecycle, aircraft tracking, receiver status',
    uid: 'soar-run-flights',
    panelRanges: [
      { start: 62, end: 65 },  // Flight Tracker (row + 3 panels)
      { start: 66, end: 69 },  // Flight Tracker Coalesce (row + 3 panels)
      { start: 70, end: 71 }   // Receiver Status (row + 1 panel)
    ]
  },
  'grafana-dashboard-run-geocoding.json': {
    title: 'SOAR Run - Geocoding',
    description: 'Pelias geocoding service metrics',
    uid: 'soar-run-geocoding',
    panelRanges: [
      { start: 5, end: 11 }    // Geocoding - Pelias (row + 6 panels)
    ]
  },
  'grafana-dashboard-run-elevation.json': {
    title: 'SOAR Run - Elevation Processing',
    description: 'Elevation lookups and AGL calculations',
    uid: 'soar-run-elevation',
    panelRanges: [
      { start: 41, end: 55 }   // Elevation Processing (row + 14 panels)
    ]
  }
};

// Create each dashboard
Object.entries(splits).forEach(([filename, config]) => {
  const newDashboard = {
    ...source,
    title: config.title,
    uid: config.uid,
    description: config.description,
    panels: []
  };

  // Extract panels for this dashboard
  let yPos = 0;
  config.panelRanges.forEach(range => {
    for (let i = range.start; i <= range.end && i < source.panels.length; i++) {
      const panel = JSON.parse(JSON.stringify(source.panels[i])); // Deep copy

      // Reset grid position
      panel.gridPos = {
        x: 0,
        y: yPos,
        w: 24,
        h: panel.type === 'row' ? 1 : 8
      };

      // Reset panel ID to avoid conflicts
      panel.id = newDashboard.panels.length + 1;

      newDashboard.panels.push(panel);
      yPos += panel.gridPos.h;
    }
  });

  // Write dashboard file
  const outputPath = path.join(outputDir, filename);
  fs.writeFileSync(outputPath, JSON.stringify(newDashboard, null, 2));
  console.log(`✓ Created ${filename} with ${newDashboard.panels.length} panels (height: ${yPos} units)`);
});

console.log('\n✅ Dashboard split complete!');
console.log('\nCreated dashboards:');
Object.keys(splits).forEach(filename => {
  console.log(`  - ${filename}`);
});
