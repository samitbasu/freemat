pitch = 3.4;
rows = 16;
sdd = 1050;
sid = 600;
mag = sdd/sid;
width = pitch*rows/mag;
h = 2;
dist_rotation = h*width;
rpms = 150;
period = 60/rpms;
dist_time = dist_rotation/period;
bagsize = 750;
spacing = 250;
window_size = bagsize+spacing;
time_per_window = window_size/dist_time;
bph = 3600/time_per_window;
