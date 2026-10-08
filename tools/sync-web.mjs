#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');
const sourceWeb = path.resolve(rootDir, '../biank/apps/desktop/dist/web-vite');
const targetWeb = path.resolve(rootDir, 'dist/web');

if (!fs.existsSync(sourceWeb)) {
  console.error(`Error: No se encontró el directorio de origen en ${sourceWeb}`);
  process.exit(1);
}

fs.rmSync(targetWeb, { recursive: true, force: true });
fs.mkdirSync(path.dirname(targetWeb), { recursive: true });
fs.cpSync(sourceWeb, targetWeb, { recursive: true });

console.log(`✓ Frontend sincronizado exitosamente desde ${sourceWeb} -> ${targetWeb}`);
