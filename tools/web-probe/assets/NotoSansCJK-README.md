# Browser CJK fallback

Unmodified NotoSansCJKjp-Regular.otf, licensed under SIL OFL 1.1; see
NotoSansCJK-OFL.txt. Source: https://github.com/notofonts/noto-cjk/tree/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/Japanese

Reused from Rustable's existing licensed asset. SHA-256:
68a3fc98800b2a27b371f2fb79991daf3633bd89309d4ffaa6946fd587f375b5
Size: 16,467,736 bytes. Covers Chinese, Japanese and Korean using Japanese
regional glyph forms. Does not provide emoji or universal script coverage.

Packaged separately from WASM and fetched from the same origin only when CJK
input/preedit is observed. The loader bounds the stream, checks its hash, and
uses a 30-second timeout. Once loaded, the font remains in the browser text
system until the page closes; it is shared by all panes.
