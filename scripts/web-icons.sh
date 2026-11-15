#!/bin/sh
# Regenerates the web PNG icons from the single vector source apps/web/icon.svg.
# Needs rsvg-convert and ImageMagick (`magick`); the generated PNGs are committed, so builds do not.
#   icon-192.png / icon-512.png     "any": the icon as drawn (rounded square, transparent corners)
#   icon-maskable-512.png           "maskable": full-bleed, content scaled into the 80% safe circle
#   apple-touch-icon.png            180x180, opaque and full-bleed (iOS rounds it itself)
set -eu
cd "$(dirname "$0")/../apps/web"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
src=icon.svg
sed 's/ rx="42"//' "$src" > "$tmp/full.svg"
if grep -q 'rx="42"' "$tmp/full.svg"; then echo 'icon.svg changed: update the corner radius handling' >&2; exit 1; fi
# Maskable: the full-bleed drawing nested at 166/192 (= 86.4%) over the same blue, so the 3x3 grid
# spans 54% of the icon and its corners stay inside the 40%-radius safe zone.
blue="$(sed -n 's/.*<rect width="192" height="192" rx="42" fill="\(#[0-9a-fA-F]*\)".*/\1/p' "$src")"
body="$(sed -e 's/^<svg[^>]*>//' -e 's/<\/svg>[[:space:]]*$//' "$tmp/full.svg")"
printf '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192"><rect width="192" height="192" fill="%s"/><svg x="13" y="13" width="166" height="166" viewBox="0 0 192 192">%s</svg></svg>\n' "$blue" "$body" > "$tmp/maskable.svg"
render() { rsvg-convert -w "$2" -h "$2" "$1" -o "$3"; }
render "$src" 192 icon-192.png
render "$src" 512 icon-512.png
render "$tmp/maskable.svg" 512 icon-maskable-512.png
render "$tmp/full.svg" 180 "$tmp/touch.png"
magick "$tmp/touch.png" -background "$blue" -alpha remove -alpha off -strip PNG24:apple-touch-icon.png
magick icon-maskable-512.png -background "$blue" -alpha remove -alpha off -strip PNG24:icon-maskable-512.png
magick icon-192.png -strip PNG32:icon-192.png
magick icon-512.png -strip PNG32:icon-512.png
ls -l icon-192.png icon-512.png icon-maskable-512.png apple-touch-icon.png
