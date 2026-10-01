# Preparation label layout, 2026-09-19

The user supplied a ZPL print photograph and confirmed 80 × 70 mm stock. Text
occupied only the upper portion. The raster renderer previously scaled fonts
from width/640 and only shrank them; it did not fill available height.

Use DPI/96 for the font baseline, then fit text up or down to the usable height.
Wrap the full header and body text rather than truncating them as fonts grow.
Keep expiration and container count together within the printable width, and
distribute remaining vertical space between rows. Preserve the configured
preprinted-header clearance. No clinical values, calculations, or legacy MDBs
are changed; this is an output layout adjustment.

New printer configurations default to 80 × 70 mm. Existing saved workstation
dimensions remain authoritative and must be set to the actual stock in Settings.
Raster regression checks cover 203, 300, and 600 dpi, header clearance, bottom
margin, and expiration/counter width. Physical output still requires checking
on the user's printer; the supplied photo cannot establish its DPI or offset.

## Requested layout, 2026-09-29

Preview and raster printing now start with HN and ward, followed by patient name
on its own row. The OncoFlow/hospital heading is removed. Drug and dose start
with "ยา: "; "in" plus diluent/volume occupies the following row. Missing, blank,
or zero infusion time displays "ตามโปรโตคอล" alongside the route. Nonzero
values retain their recorded text. This changes presentation, not calculations.

Migration 020 freezes the order ward name when new output snapshots are created.
Existing snapshots retain an unknown ward (displayed as —); they are not
backfilled from mutable current orders. Reprints preserve the captured ward.

Label row captions use วิธีให้ยา:, การเก็บยา:, and คำเตือน:. Withdrawal is
displayed as ดูดยา: in both preview and raster printing.

## Workstation font selection

Settings → Hardware lists locally readable TrueType/OpenType faces (including
collections) from Windows Fonts and the current user's Microsoft/Windows/Fonts
directory. Only faces containing Thai and Latin label characters are offered.
The selected full font name is stored in workstation localStorage; older settings
retain the automatic default. Refresh printers / fonts rescans after installation.
The WebView preview loads the exact local face with FontFace; the Rust renderer
resolves the same discovered face and collection index for test and final prints.
Missing selected fonts stop printing instead of silently substituting a font.
No font download or external database is involved.

LAN and host mode request frozen output from the server, render on the printing
workstation, then acknowledge only after spooler acceptance. A render failure
cancels the pending print before submission and produces no print audit. A spooler
or acknowledgement failure keeps the existing uncertain-print handling. The
server keeps its raster response for older clients; use matching app versions.

## Per-row text styles, 2026-10-01

Settings → Hardware exposes independent bold and underline toggles for all ten
label rows, including separate drug and diluent rows. Styles are saved per
workstation and default to normal text without underlining for older settings.
Preview applies each row style and refits the content. The raster renderer applies
synthetic bold strokes and an underline to each wrapped text line, including both
expiration and container number. The selected local font remains authoritative.
