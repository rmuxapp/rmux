import base64
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

from release_metadata import SPARKLE, validate_appcast


class ReleaseMetadataTests(unittest.TestCase):
    def test_rejects_mismatched_or_incomplete_update_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / 'rmux-1.2.3-macos-arm64.zip'
            archive.write_bytes(b'fixture archive')
            feed = root / 'appcast.xml'
            item = ET.Element('item')
            ET.SubElement(item, SPARKLE + 'version').text = '1.2.3'
            ET.SubElement(item, SPARKLE + 'shortVersionString').text = '1.2.3'
            ET.SubElement(item, SPARKLE + 'fullReleaseNotesLink').text = 'https://github.com/rmuxapp/rmux/releases/tag/v1.2.3'
            ET.SubElement(item, 'description', {SPARKLE + 'format': 'markdown'}).text = '# Changes\n\nUnicode 世界 & updates'
            ET.SubElement(item, 'enclosure', {
                'url': f'https://github.com/rmuxapp/rmux/releases/download/v1.2.3/{archive.name}',
                'length': str(archive.stat().st_size),
                SPARKLE + 'edSignature': base64.b64encode(bytes(64)).decode(),
            })
            def write(candidate):
                rss = ET.Element('rss')
                ET.SubElement(rss, 'channel').append(candidate)
                ET.ElementTree(rss).write(feed, encoding='utf-8')
            write(item)
            validate_appcast(feed, '1.2.3', archive)
            for tag, attribute, value in [
                (SPARKLE + 'version', None, '0.0.1'),
                (SPARKLE + 'shortVersionString', None, '0.0.1'),
                ('enclosure', 'url', 'https://example.com/wrong.zip'),
                ('enclosure', 'length', '0'),
                ('enclosure', SPARKLE + 'edSignature', ''),
                ('description', None, '  '),
                (SPARKLE + 'fullReleaseNotesLink', None, 'https://example.com'),
            ]:
                with self.subTest(field=(tag, attribute)):
                    candidate = ET.fromstring(ET.tostring(item))
                    element = candidate.find(tag)
                    if attribute:
                        element.set(attribute, value)
                    else:
                        element.text = value
                    write(candidate)
                    with self.assertRaises(ValueError):
                        validate_appcast(feed, '1.2.3', archive)


if __name__ == '__main__':
    unittest.main()
