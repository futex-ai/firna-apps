# X Icon

`icon.svg` is the source of truth: X's official mark, in white, centred on a
black rounded-square tile so the icon carries its own background instead of
relying on whatever surface a product view places behind it. The mark occupies
roughly half the tile width, which keeps the clear space around it consistent
with the other packaged app icons.

`icon.png` is the 128x128 rasterisation of that source, and `icon.png.base64` is
the reproducible text encoding embedded in `manifest.yaml`. Product activity
treatments use the declared black `#000000` and white `#FFFFFF` pair.

Regenerate all three after changing `icon.svg`:

```bash
magick -background none -density 2400 apps/x/assets/icon.svg \
  -resize 1024x1024 -filter Lanczos -resize 128x128 -strip \
  PNG32:apps/x/assets/icon.png
{ base64 -i apps/x/assets/icon.png | tr -d '\n'; printf '\n'; } \
  > apps/x/assets/icon.png.base64
```

Then replace the manifest's `icon.data_base64` value with the contents of
`icon.png.base64`. `cargo xtask check` fails when the PNG, the sidecar, and the
manifest disagree, or when the icon neither paints its own background nor keeps
clear space around its mark.

Source: <https://about.x.com/en/who-we-are/brand-toolkit>
