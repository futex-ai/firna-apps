# Slack Image Assets

`icon.svg` is the source for Slack's package image. `icon.png` is the rendered
catalog image and `icon.png.base64` is the exact payload embedded by the
top-level manifest `icon`.

`tools/` uses the repository's optional command-image convention. Each
declared override has a `<tool-name>.svg` source, a 128x128 PNG, and a base64
sidecar whose contents are embedded in the matching `tools[].icon`. Slack
intentionally has no command assets for `slack_list_channels` or
`slack_read_channel_history`; those tools use the package image.

Regenerate the declared command images from the repository root with
`rsvg-convert` from `librsvg`:

```sh
for tool in slack_send_message slack_search_messages; do
  rsvg-convert --width 128 --height 128 --keep-aspect-ratio \
    "apps/slack/assets/tools/${tool}.svg" \
    > "apps/slack/assets/tools/${tool}.png"
  { base64 -i "apps/slack/assets/tools/${tool}.png" | tr -d '\n'; printf '\n'; } \
    > "apps/slack/assets/tools/${tool}.png.base64"
done
```

Copy each sidecar's full single-line value into its manifest declaration, then
run `python3 scripts/repository_audit.py --base origin/main` and
`cargo xtask check`. The audit verifies the manifest, PNG, and sidecar bytes
remain identical.
