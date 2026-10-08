"""Verify every staged resource in the actual macOS updater archive, without installing it."""
import hashlib
import json
import sys
import tarfile

archive_path = sys.argv[1]
prefix = "Biank.app/Contents/Resources/runtime/"
manifest = None
with tarfile.open(archive_path, "r|gz") as archive:
    for member in archive:
        if member.name.removeprefix("./") == prefix + "resources-manifest.json":
            with archive.extractfile(member) as stream:
                manifest = json.load(stream)["files"]
            break
if manifest is None:
    raise ValueError("Manifest ausente del updater")
checked = set()
with tarfile.open(archive_path, "r|gz") as archive:
    for member in archive:
        name = member.name.removeprefix("./")
        if not name.startswith(prefix):
            continue
        relative = name[len(prefix):]
        if relative not in manifest:
            continue
        expected = manifest[relative]
        if "sha256" in expected:
            if not member.isfile():
                raise ValueError(relative + ": recurso regular requerido")
            with archive.extractfile(member) as stream:
                if hashlib.file_digest(stream, "sha256").hexdigest() != expected["sha256"]:
                    raise ValueError(relative + ": contenido modificado")
        elif not member.issym() or member.linkname != expected["link"]:
            raise ValueError(relative + ": enlace modificado")
        checked.add(relative)
missing = set(manifest) - checked
if missing:
    raise ValueError("Recursos ausentes: " + str(sorted(missing)[:8]))
print("Integridad del tar.gz macOS: PASS (" + str(len(checked)) + " recursos)")
