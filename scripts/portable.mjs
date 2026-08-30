import fs from 'node:fs'
import fsp from 'node:fs/promises'
import { createRequire } from 'node:module'
import path from 'node:path'

import AdmZip from 'adm-zip'

const target = process.argv.slice(2)[0]
const ARCH_MAP = {
  'x86_64-pc-windows-msvc': 'x64',
  'aarch64-pc-windows-msvc': 'arm64',
}

const PROCESS_MAP = {
  x64: 'x64',
  arm64: 'arm64',
}
const arch = target ? ARCH_MAP[target] : PROCESS_MAP[process.arch]
/// Script for ci
/// 打包绿色版/便携版 (only Windows)
async function resolvePortable() {
  if (process.platform !== 'win32') return

  const releaseCandidates = target
    ? [`./target/${target}/release`, `./src-tauri/target/${target}/release`]
    : ['./target/release', './src-tauri/target/release']
  const releaseDir = releaseCandidates.find((dir) => fs.existsSync(dir))
  if (!releaseDir) {
    throw new Error(
      `could not find the release dir (checked: ${releaseCandidates.join(', ')})`,
    )
  }
  const configDir = path.join(releaseDir, '.config')

  await fsp.mkdir(configDir, { recursive: true })
  if (!fs.existsSync(path.join(configDir, 'PORTABLE'))) {
    await fsp.writeFile(path.join(configDir, 'PORTABLE'), '')
  }
  const zip = new AdmZip()

  zip.addLocalFile(path.join(releaseDir, 'clash-verge.exe'))
  zip.addLocalFile(path.join(releaseDir, 'verge-mihomo.exe'))
  zip.addLocalFile(path.join(releaseDir, 'verge-mihomo-alpha.exe'))
  zip.addLocalFolder(path.join(releaseDir, 'resources'), 'resources')
  zip.addLocalFolder(configDir, '.config')

  const require = createRequire(import.meta.url)
  const packageJson = require('../package.json')
  const { version } = packageJson
  const zipFile = `Clash.Verge_${version}_${arch}_portable.zip`
  zip.writeZip(zipFile)
  console.log('[INFO]: create portable zip successfully')
}

resolvePortable().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
