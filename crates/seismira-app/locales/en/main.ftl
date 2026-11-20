app-title = Seismira
disclaimer = This is an educational simulation. For real earthquake information, follow official agencies such as AFAD, Kandilli, JMA or USGS.

disclaimer-close = Close notice

menu-view = View
menu-disclaimer = Educational notice
menu-shake-grid = Shaking area
menu-fault-trace = Fault trace at end of simulation
menu-reduce-motion = Reduce motion
menu-text-test = Text test
menu-language = Language

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
ranges-note = Validity ranges come from the published ranges of the ground-motion models (crust: CY14, subduction: Parker et al. 2020).
start-needs-epicenter = Choose an epicenter first.

issue-mw = For this setting and fault type, magnitude must be between { $min } and { $max }.
issue-depth = For this setting, focal depth must be between { $min } and { $max } km.
issue-interface = A subduction interface event can only be a reverse fault.
issue-deep = Focus deeper than { $max } km: the ground-motion model is not valid at this depth. Waves are shown; intensity is not computed.
issue-geometry = Invalid value (dip must be 0–90°, latitude −90–90°).
issue-ztor = The top of the rupture is { $ztor } km deep; the ground-motion model is valid up to { $max } km. Reduce the focal depth or increase the magnitude.
issue-rupture-size = The rupture-size relation is supported by data up to Mw { $max }; for larger earthquakes fault length and width are rough estimates.

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
city-p-arrived = P wave arrived: { $city }
city-shindo = { $city }: Shindo { $shindo }
region-shindo = { $region }: Shindo { $shindo } (highest: { $city })
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
model-limit = Estimate limit: { $km } km from fault

# Erken uyarı (Faz 4)
early-warning = Early warning (hypothetical system): { $regions }
early-warning-more = { $regions } and { $n } more regions
tip-lead-time = Warning time: { $s } s
tip-blind-zone = Blind zone: warning arrives after shaking

# Region card and closing the simulation (phase 4)
button-stop = Close simulation
running-hint = While a simulation is open, clicking a region on the map opens its card. Close the simulation to pick a new epicenter.
region-max = Highest: Shindo { $s }
region-max-city = Highest: Shindo { $s } ({ $city })
region-typical = Widespread: Shindo { $s } (at least 10% of the built-up area)
region-arrivals = First P: T+{ $p }, first S: T+{ $s }
region-lead = Early warning: { $s } s before the first S in the region
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
tip-backarc = Backarc: the model was built for forearc sites only; the estimate is probably too high.
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
