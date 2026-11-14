app-title = Seismira
disclaimer = This is an educational simulation. For real earthquake information, follow official agencies such as AFAD, Kandilli, JMA or USGS.

disclaimer-close = Close notice

menu-view = View
menu-disclaimer = Educational notice
menu-text-test = Text test
menu-language = Language

panel-settings = Earthquake settings
button-start = Start

panel-notifications = Notifications
notifications-empty = No notifications yet.

timeline = Timeline

text-test-title = Multilingual text test
text-test-note = In right-to-left scripts (Arabic, Hebrew), the word order of multi-word text is currently wrong; known issue, see docs/arayuz.md.

map-loading = Loading map…
map-missing = Map data not found. Run this command in the repository root:
epicenter = Epicenter
epicenter-hint = Click the map to choose the epicenter. Drag to pan, scroll to zoom.

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
ranges-note = Validity ranges are provisional; they will be finalized as ground-motion models are added.
start-needs-epicenter = Choose an epicenter first.

issue-mw = For this setting and fault type, magnitude must be between { $min } and { $max }.
issue-depth = For this setting, focal depth must be between { $min } and { $max } km.
issue-interface = A subduction interface event can only be a reverse fault.
issue-deep = Deep focus (deeper than 300 km): ground-motion models are not valid; intensity results will be rough estimates.
issue-geometry = Invalid value (dip must be 0–90°, latitude −90–90°).

# Timeline
play = Play (Space)
pause = Pause (Space)
restart = Back to start
next-event = Jump to next event (origin, waves surfacing, reaching 100°…)
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
