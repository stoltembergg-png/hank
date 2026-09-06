# Installer contract — PR-267

PR-267 defines the testable installer boundary without publishing packages or claiming platform
support beyond evidence from the declared CI matrix.

The manifest declares Linux AppImage, Windows NSIS, and macOS aarch64 DMG inputs. The offline
contract verifies canonical package paths, artifact digest/platform identity, clean install,
launch-state prerequisites, uninstall behavior, profile preservation, and migration-before-use.

Installer metadata cannot execute shell, embed secrets, delete profiles by default, or bypass
signature/digest verification. Real package publication and unattended updates remain outside
this increment.
