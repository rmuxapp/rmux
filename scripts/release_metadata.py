"""Validate update metadata against the exact archive about to be published."""
import base64
import xml.etree.ElementTree as ET

SPARKLE = '{http://www.andymatuschak.org/xml-namespaces/sparkle}'


def validate_appcast(path, version, archive):
    items = ET.parse(path).findall('./channel/item')
    matching = [item for item in items if item.findtext(SPARKLE + 'version') == version]
    if len(matching) != 1:
        raise ValueError('Appcast must contain exactly one item for this release')
    item = matching[0]
    if item.findtext(SPARKLE + 'shortVersionString') != version:
        raise ValueError('Appcast display version does not match the release')
    enclosure = item.find('enclosure')
    expected_url = f'https://github.com/rmuxapp/rmux/releases/download/v{version}/{archive.name}'
    if enclosure is None or enclosure.get('url') != expected_url:
        raise ValueError('Appcast does not point to the expected release ZIP')
    if enclosure.get('length') != str(archive.stat().st_size):
        raise ValueError('Appcast archive length does not match the release ZIP')
    signature = base64.b64decode(enclosure.get(SPARKLE + 'edSignature', ''), validate=True)
    if len(signature) != 64:
        raise ValueError('Appcast is missing a well-formed Ed25519 signature')
    # This checks metadata consistency, not the cryptographic signature itself.
    notes = item.find('description')
    if notes is None or not ''.join(notes.itertext()).strip():
        raise ValueError('Appcast is missing embedded release notes')
    expected_notes = f'https://github.com/rmuxapp/rmux/releases/tag/v{version}'
    if item.findtext(SPARKLE + 'fullReleaseNotesLink') != expected_notes:
        raise ValueError('Appcast full release notes link does not match this version')
