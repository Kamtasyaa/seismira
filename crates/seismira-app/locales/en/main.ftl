app-title = Seismira
disclaimer = This is an educational simulation. For real earthquake information, follow official agencies such as AFAD, Kandilli, JMA or USGS.

disclaimer-close = Close notice

menu-view = View
menu-disclaimer = Educational notice
menu-shake-grid = Shaking area
menu-fault-trace = Fault trace at end of simulation
menu-liquefaction = Liquefaction hazard (after shaking)
menu-landslide = Landslide hazard (after shaking)
menu-tsunami = Tsunami warnings
menu-reduce-motion = Reduce motion
menu-scale = Intensity scale
scale-shindo = JMA Shindo (default)
scale-mmi = Modified Mercalli (MMI, USGS colours)
scale-note = Map, city boxes, region colours, notifications and cards use the selected scale. Whether an early warning is issued, and where, always follows the JMA Shindo criterion.
mmi-legend = MMI (Worden et al. 2012)
level-shindo = Shindo { $v }
level-mmi = MMI { $v }
menu-quality = Quality
quality-low = Low (up to 512 cells, fast)
quality-medium = Medium (up to 1024 cells)
quality-high = High (up to 2048 cells, default)
quality-ultra = Ultra (2048 cells, damage at full resolution)
quality-note = Resolution of the shaking grid and damage estimate. Computed in the background; an open simulation is recomputed.
menu-text-test = Text test
menu-language = Language
menu-help = Help
menu-sources = Data sources and licenses
sources-title = Data Sources and Licenses
sources-intro = Seismira is free software licensed under GPL-3.0-or-later. Data shipped with the application is distributed under its own license; attributions are listed below. "Modified": the data was processed (format, resolution, derivation) before use.
sources-license = License:
sources-modified = · modified
sources-not-distributed = · not in the distributed package (local builds only)
sources-missing = The data sources file (data/SOURCES.toml) could not be read.

panel-settings = Earthquake settings
button-start = Start

panel-notifications = Notifications
notifications-empty = No notifications yet.

text-test-title = Multilingual text test
text-test-note = In right-to-left scripts (Arabic, Hebrew), the word order of multi-word text is currently wrong; known issue, see docs/arayuz.md.

map-loading = Loading map…
map-missing = Map data not found. Run this command in the repository root:

# Sayı ve koordinat biçimi
decimal-separator = .
hemi-n = N
hemi-s = S
hemi-e = E
hemi-w = W

# Earthquake settings
setting = Tectonic setting
setting-crust = Shallow crust
setting-interface = Subduction interface
setting-intraslab = Intraslab
fault = Fault type
fault-ss = Strike-slip
fault-normal = Normal
fault-reverse = Reverse
mw = Magnitude (Mw)
depth = Focal depth (km)
lat = Latitude
lon = Longitude
strike = Strike (°)
dip = Dip (°)
rake = Rake (°)
fault-geometry = Fault geometry
ranges-note = Validity ranges come from the published ranges of the ground-motion models (crust: average of ASK14, BSSA14, CB14, CY14; subduction: Parker et al. 2020).
start-needs-epicenter = Choose an epicenter first.

issue-mw = For this setting and fault type, magnitude must be between { $min } and { $max }.
issue-depth = For this setting, focal depth must be between { $min } and { $max } km.
issue-interface = A subduction interface event can only be a reverse fault.
issue-deep = Focus deeper than { $max } km: the ground-motion model is not valid at this depth. Waves are shown; intensity is not computed.
issue-geometry = Invalid value (dip must be 0–90°, latitude −90–90°).
issue-ztor = The top of the rupture is { $ztor } km deep; the ground-motion model is valid up to { $max } km. Reduce the focal depth or increase the magnitude.
issue-rupture-size = The rupture-size relation is supported by data up to Mw { $max }; for larger earthquakes fault length and width are rough estimates.
issue-rupture-invalid = Rupture length must be 1–{ $l } km and width 1–{ $w } km.
issue-rupture-clamped = Rupture width does not fit above the seismogenic base; clipped to { $w } km.
issue-rupture-area = Rupture area is { $r } times the typical value for this magnitude (Wells and Coppersmith 1994, outside ±2.5σ); results may differ from expectations.
issue-stable = The epicentre is in a stable continental region (USGS STREC). The ground-motion models were built for active crust; shaking may reach farther here and results are not representative.
issue-trace = Invalid fault trace: each segment must be at least 1 km, the total at most { $l } km; segment dip 1–90°.

# Timeline
play = Play (Space)
pause = Pause (Space)
restart = Back to start
next-event = Jump to next notification (early warning, region Shindo values, P wave reaching cities…)
step-back = Back 10 seconds (←; Shift+← 1 minute)
step-forward = Forward 10 seconds (→; Shift+→ 1 minute)
speed = Speed
time-elapsed = T+{ $t }

# Waves and events
wave-p = P wave
wave-s = S wave
event-origin = Earthquake: M{ $mw } at { $depth } km depth
event-surfaced-p = P wave reached the surface
event-surfaced-s = S wave reached the surface
event-limit-p = P wave reached { $deg }°; beyond is the P shadow zone
event-limit-s = S wave reached { $deg }°; end of simulation
traveltime-missing = Travel-time table not found (data/traveltime/ak135.ttt).

# Intensity and cities (Phase 3)
shindo-legend = Shindo (JMA)
percent = { $v }%
city-p-arrived = P wave arrived: { $city }
city-shindo = { $city }: { $shindo }
region-shindo = { $region }: { $shindo } (highest: { $city })
tip-shindo = Shindo { $s } (instrumental intensity { $i }; likely range { $lo } – { $hi })
tip-mmi = MMI { $roman } ({ $v }; likely range { $lo } – { $hi })
tip-pga = PGA: { $v } %g
tip-pgv = PGV: { $v } cm/s
tip-arrivals = P: T+{ $p }, S: T+{ $s } (S − P: { $sp } s)
tip-distance = Distance to fault: { $d } km
tip-outside-model = The ground-motion model is not valid at this distance.
tip-vs30-map = Site: Vs30 ≈ { $v } m/s (USGS map)
tip-vs30-nearby = Site: Vs30 ≈ { $v } m/s (nearest land cell)
tip-vs30-built = Site: Vs30 ≈ { $v } m/s (USGS map, weighted by built-up area)
tip-vs30-reference = Site: reference Vs30 = { $v } m/s (no map value)
tip-vs30-clamped = Clamped to the model limit: { $v } m/s
tip-vs30-sd = Site uncertainty: Vs30 ×/÷ { $f } → intensity ±{ $d }
no-gmpe = No ground-motion model at this depth; only waves are shown.
no-gmpe-short = No intensity estimate for this earthquake.
advanced = Advanced
hypo-along = Hypocenter, along strike
hypo-down = Hypocenter, down dip
rupture-size = Rupture: { $l } × { $w } km, top edge at { $ztor } km depth
rupture-manual = Enter rupture size manually
rupture-length = Rupture length
rupture-width = Rupture width
trace-info = Multi-segment fault trace: { $n } segments, { $l } km in total. Strike, length and the hypocentre position along strike come from the trace.
trace-remove = Remove fault trace (back to a single plane)
trace-draw = Draw fault trace (on the map)
trace-redraw = Redraw fault trace
trace-drawing = Drawing fault trace: { $n } points. Click the map to add a point; double-click or Enter finishes, Esc cancels, Backspace removes the last point. The fault dips to the right of the drawing direction.
trace-finish = Finish
trace-undo = Remove last point
trace-cancel = Cancel
trace-segments = Segments ({ $n })
trace-seg-header = Segment: length · strike
trace-seg-dip = Own dip (°)
trace-seg-own-dip = Give this segment a dip different from the common dip
issue-epicenter-off-trace = The epicentre is { $km } km from the fault trace; the rupture is assumed to start at the nearest point of the trace.
model-limit = Estimate limit: { $km } km from fault

# Erken uyarı (Faz 4)
early-warning = Early warning (hypothetical system): { $regions }
early-warning-more = { $regions } and { $n } more regions
tip-lead-time = Warning time: { $s } s
tip-blind-zone = Blind zone: warning arrives after shaking

# Region card and closing the simulation (phase 4)
button-stop = Close simulation
running-hint = While a simulation is open, clicking a region on the map opens its card. Close the simulation to pick a new epicenter.
region-max = Highest: { $s }
region-max-city = Highest: { $s } ({ $city })
region-typical = Widespread: { $s } (at least 10% of the built-up area)
region-arrivals = First P: T+{ $p }, first S: T+{ $s }
region-lead = Early warning: { $s } s before the first S in the region
region-liquefaction = Liquefaction: ~{ $km2 } km² of area affected (likely { $lo }–{ $hi })
region-landslide = Landslides: ~{ $km2 } km² of area affected (likely { $lo }–{ $hi })
region-tsunami = Tsunami: { $cat }, { $h }, first wave { $eta }
ground-letter-liquefaction = LQ
ground-letter-landslide = LS
ground-name-liquefaction = Liquefaction hazard
ground-name-landslide = Landslide hazard
ground-legend-note = Square: ~{ $km } km area; USGS yellow alert threshold
ground-notice = [{ $letter }] { $name }: { $region }
ground-notice-unnamed = [{ $letter }] { $name }
region-blind = Early warning: blind zone (the alert comes after the first S)
region-not-covered = Outside early-warning coverage (below Shindo 4)

## Slab2 slab geometry and Parker et al. (2020) region (Phase 5)
slab-none = According to Slab2 there is no subducting slab beneath this point, so a subduction earthquake may not be expected here (Slab2 does not cover every zone, e.g. the Cyprus arc). The estimate uses global coefficients without a forearc / backarc distinction.
slab-missing = Slab2 data not found; the subduction model uses global coefficients.
slab-info = Slab ({ $zone }, Slab2): surface at { $d } km, strike { $strike }°, dip { $dip }°
slab-region = Parker et al. (2020) region: { $region }
slab-apply = Apply slab geometry
slab-too-deep = The slab is { $d } km deep here; an interface earthquake can be at most { $max } km deep.
slab-above = The focus is above the slab surface ({ $d } km); an intraslab earthquake must be inside the slab.
tip-backarc = Backarc: the model was built from forearc data; the estimate may be too high here.
parker-region-global = global
parker-region-alaska = Alaska
parker-region-aleutian = Aleutian Islands
parker-region-cascadia = Cascadia
parker-region-cam_n = Central America and Mexico (north)
parker-region-cam_s = Central America and Mexico (south)
parker-region-japan_pac = Japan, Pacific Plate
parker-region-japan_phi = Japan, Philippine Sea Plate
parker-region-sa_n = South America (north)
parker-region-sa_s = South America (south)
parker-region-taiwan_e = Taiwan (northeast)
parker-region-taiwan_w = Taiwan (southwest)
slab-zone-alu = Alaska-Aleutians
slab-zone-cal = Calabria
slab-zone-cam = Central America
slab-zone-car = Caribbean
slab-zone-cas = Cascadia
slab-zone-cot = Cotabato
slab-zone-hal = Halmahera
slab-zone-hel = Hellenic Arc
slab-zone-him = Himalaya
slab-zone-hin = Hindu Kush
slab-zone-izu = Izu-Bonin
slab-zone-ker = Kermadec-Tonga
slab-zone-kur = Kamchatka-Kuril-Japan
slab-zone-mak = Makran
slab-zone-man = Manila Trench
slab-zone-mue = Muertos Trough
slab-zone-pam = Pamir
slab-zone-phi = Philippines
slab-zone-png = New Guinea
slab-zone-puy = Puysegur
slab-zone-ryu = Ryukyu-Nankai
slab-zone-sam = South America
slab-zone-sco = Scotia Sea
slab-zone-sol = Solomon Islands
slab-zone-sul = Sulawesi
slab-zone-sum = Sumatra-Java
slab-zone-van = Vanuatu

## Estimated building damage (Phase 6, ESRM20)
region-damage-head = Estimated building damage (ESRM20, { $n } buildings in the region):
region-damage-heavy = Extensive damage or collapse: ~{ $n } buildings ({ $p }%; likely range { $lo }–{ $hi }%)
region-damage-heavy-none = Extensive damage or collapse: not expected (fewer than 1 building)
region-damage-moderate = Moderate damage: ~{ $n } buildings ({ $p }%)
region-damage-nodata = No building data for this region.
region-damage-nodata-nogem = No building data for this region (ESRM20 covers Europe and Türkiye only; the GEM package for the rest of the world is not installed).
region-loss-head = Estimated structural damage (GEM, value-weighted average of the region's buildings):
region-loss-ratio = Mean damage ratio: { $p }% (likely range { $lo }–{ $hi }%)
region-loss-none = Mean damage ratio: negligible
tip-damage = Estimated extensive damage or collapse: { $p }% (likely range { $lo }–{ $hi }%)
tip-damage-note = (using the region's building mix, not the city's own data)

# Tsunami (Faz 8)
tsunami-cat-major = Major tsunami warning
tsunami-cat-warning = Tsunami warning
tsunami-cat-advisory = Tsunami advisory
tsunami-issued = { $cat }: { $regions }
tsunami-heights = Estimated maximum runup (rough): { $list }
tsunami-info-small = Tsunami information: earthquake too small for a tsunami threat (PTWC criteria)
tsunami-info-deep = Tsunami information: earthquake too deep (≥ 100 km), no threat expected (PTWC criteria)
tsunami-info-inland = Tsunami information: earthquake inland, no threat expected (PTWC criteria)
tsunami-arrived = Tsunami arrived: { $region } { $h }
tsunami-height-suffix = (~{ $m } m)
tsunami-legend-title = Tsunami (JMA categories, ~3 min after the quake)
tsunami-legend-major = Major tsunami warning: > 3 m
tsunami-legend-warning = Tsunami warning: 1–3 m
tsunami-legend-advisory = Advisory: 0.2–1 m or threat zone (no height)
tsunami-legend-note = Heights are rough estimates (Riquelme et al. 2015); none for bays, sheltered coasts, small islands
tsunami-tip = { $place }: { $cat }, { $h }, first wave { $eta }
tsunami-no-height = height not estimated
tsunami-height = estimated ~{ $m } m
tsunami-unnamed = Coast

# Hazır senaryolar (Faz 10)
scenario = Preset scenario
scenario-custom = Custom (manual)
scenario-note = From the USGS event record and ShakeMap rupture model. Earthquakes whose rupture is mapped segment by segment use a multi-segment fault trace (branches excluded), the others a single-plane equivalent. Changing any value makes the scenario "Custom".
scenario-kahramanmaras2023 = 2023 Kahramanmaraş (Pazarcık), Mw 7.8 — first mainshock only
scenario-izmit1999 = 1999 İzmit (Gölcük), Mw 7.6
scenario-kumamoto2016 = 2016 Kumamoto (Japan), Mw 7.0
scenario-ridgecrest2019 = 2019 Ridgecrest (USA), Mw 7.1
scenario-tohoku2011 = 2011 Tōhoku (Japan), Mw 9.1
scenario-noto2024 = 2024 Noto Peninsula (Japan), Mw 7.5
scenario-maule2010 = 2010 Maule (Chile), Mw 8.8
scenario-puebla2017 = 2017 Puebla (Mexico), Mw 7.1

## Your own data / export
menu-data = Data
menu-user-load = Load your own data…
menu-export = Export results…
tip-vs30-user = Site: Vs30 ≈ { $v } m/s (your uploaded map)
user-title = Load your own data
user-intro = Three kinds of file can be loaded. Fault trace (.geojson): a line that becomes the earthquake's fault trace. Vs30 site map (.tif, geographic GeoTIFF, values in m/s): replaces the application's site map on land where it has data. Building inventory (.csv with a header; columns: lon / boylam, lat / enlem, buildings / bina, optional name): shaking and the expected number of heavily damaged buildings are computed for each point.
user-pick = Choose file…
user-pick-title = Seismira: choose a fault trace, Vs30 map or building inventory
user-pick-filter = Fault trace, Vs30, inventory
user-path = or file path
user-load = Load
user-drop = You can also drag and drop the file onto the window (not supported on some Linux desktops, e.g. Wayland; paste the path instead).
user-loading = Reading…
user-err = Could not read { $file }:
user-err-read = the file could not be opened.
user-err-json = not valid JSON.
user-err-no-line = it contains no line (LineString / MultiLineString).
user-err-not-lonlat = coordinates are not longitude / latitude (degrees); convert the file to EPSG:4326.
user-err-tiff = the TIFF could not be read.
user-err-too-big = too large (at most 100 million cells).
user-err-no-geo = no location information (GeoTIFF tags).
user-err-projected = coordinates are not longitude / latitude (degrees); convert the file to EPSG:4326.
user-err-bands = it has several bands; a single-band Vs30 map is needed.
user-err-no-vs30 = it contains no valid Vs30 value (100–3000 m/s).
user-err-csv-empty = no readable rows.
user-err-csv-columns = longitude, latitude and building count columns not found. Columns found:
user-err-ext = unrecognised file type (must be .geojson, .tif or .csv).
user-ok-fault = { $file }: fault trace with { $n } segments loaded.
user-ok-vs30 = { $file }: { $w } × { $h } cells, { $valid } valid Vs30 values loaded.
user-ok-inventory = { $file }: { $n } points, { $b } buildings in total loaded ({ $skipped } rows skipped).
user-vs30-loaded = Vs30 map: { $file }, { $valid } valid cells; extent { $w }…{ $e } E, { $s }…{ $n } N (blue dashed frame on the map).
user-vs30-none = No Vs30 map loaded (the application's USGS map is used).
user-remove = Remove
user-inv-loaded = Building inventory: { $file }, { $n } points, { $b } buildings (blue squares on the map).
user-inv-result = Expected heavily damaged or collapsed buildings: { $m } (likely { $lo }–{ $hi }); out of { $c } buildings whose damage could be computed.
user-inv-partial = Damage could not be computed for some points: building mix data (ESRM20) exists only for Turkey and Europe, or the point is beyond the model limit.
user-inv-wait = Computing…
user-inv-run = Start an earthquake to see the result.
user-inv-none = No building inventory loaded.
export-title = Export results
export-intro = Writes the open simulation's results to files that open in QGIS / ArcGIS: shaking maps (GeoTIFF: JMA intensity, PGV, PGA), city and region tables (CSV), fault trace and ground-failure squares (GeoJSON), and your building inventory result if loaded. Coordinates are EPSG:4326.
export-where = Folder: { $dir }/seismira-<date-time>
export-need-sim = Start an earthquake first.
export-still-running = Still computing; if you export now, unfinished layers (damage, ground failure) may be missing.
export-button = Export
export-done = Written: { $dir }
export-open = Open folder

## Response spectrum
menu-spectrum = Response spectrum…
spectrum-title = Response spectrum
spectrum-need-sim = Start an earthquake first, then right-click a point on the map.
spectrum-pick = Right-click a point on the map to plot its response spectrum.
spectrum-site = Site { $lat } N, { $lon } E; distance to fault { $rrup } km; Vs30 { $vs30 } m/s ({ $src }). Right-click the map for a new site.
spectrum-vs30-user = your uploaded map
spectrum-vs30-map = USGS map
spectrum-vs30-reference = reference site, no map value
spectrum-out-of-model = This site is beyond the ground-motion model's limit; no spectrum computed.
spectrum-tbdy = For the TBDY-2018 (Turkish code) design spectrum, enter the map spectral acceleration coefficients (AFAD tdth.afad.gov.tr, for the hazard level you want, e.g. DD-2): short-period S_S and 1 s S_1.
spectrum-class = Site class
spectrum-class-auto = { $c } (from Vs30)
spectrum-zf = For ZF sites the design spectrum requires a site-specific analysis (TBDY 16.5); not drawn.
spectrum-design-values = S_DS = { $sds } g, S_D1 = { $sd1 } g, T_A = { $ta } s, T_B = { $tb } s (TBDY Eq. 2.1–2.3).
spectrum-tmax = Period range
spectrum-axis-t = Period T (s)
spectrum-axis-sa = Spectral acceleration (g)
spectrum-legend-median = prediction (median)
spectrum-legend-band = likely range (±1σ)
spectrum-legend-design = TBDY-2018 design spectrum
spectrum-note = The prediction is for this one earthquake scenario; the design spectrum is the hazard from all possible earthquakes in the region (probabilistic). They are not the same thing; the comparison gives an idea. The prediction is RotD50 of the two horizontal components, 5% damping.
spectrum-save = Save spectrum (CSV)
spectrum-hover = T = { $t } s: { $m } g (likely { $lo }–{ $hi })
spectrum-hover-design = ; TBDY { $d } g
