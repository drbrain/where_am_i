# Where am I?

## What is this?

This project aims to replace enough of gpsd to allow me to feed timestamp and
PPS data into the [Network Time Protocol](https://www.ntp.org) daemon, and to
allow tracking of GPS statistics.

## How do I use this?

Presently this works on Linux using a serial-port GPS with the PPS signal
provided through the PPS ioctls.

I used a Raspberry Pi 4 and with these overlays in `/boot/config.txt`:

```
# Enable PPS on gpio pin 18
dtoverlay=pps-gpio,gpiopin=18

# Use hardware UART for gpio pins instead of bluetooth
dtoverlay=disable-bt
```

Other Raspberry Pi versions may need different overlays to enable PPS or allow
the GPIO to use the hardware UART.

I attached the GPS to GPIO pins 14 and 15 (pins 8 and 10).  I attached the GPS
PPS output to GPIO pin 18 (pin 12).  My GPS also has a dedicated reset pin, so
I attached that to GPIO pin 17 (pin 11).

Note that "GPIO pins" and "pins" have different numbering, check `pinout` on
your Raspberry Pi or the reference documentation.

The [GPSD_JSON
refclock](https://www.eecis.udel.edu/~mills/ntp/html/drivers/driver46.html)
requires `/dev/gps0` to be a symlink to the GPS UART, so I used `udev` to
create it by adding a `/etc/udev/rules.d/50-gps.rules`:

```
KERNEL=="ttyAMA0", SYMLINK+="gps0"
```

`/dev/ttyAMA0` is the default name of the hardware UART.  After restarting the
`udev` service you should see `/dev/gps0` as a symlink to `/dev/ttyAMA0`.

Presently only enough of the gpsd protocol to support the GPSD_JSON refclock is
implemented.  This includes the `?VERSION` command and enough of the `?WATCH`
commands for ntpd to stream `TOFF` and `PPS` events.  (The refclock driver says
it requires `TPV` events, but it accepts either `PPS` or `TPV` events.)

Add the driver to `/etc/ntp.conf` with:

```
server 127.127.46.0 prefer
fudge 127.127.46.0 time1 0.105 time2 0.0
```

You will need to adjust the `time1` and `time2` parameters to get ntpd to accept
the clock as a "good ticker".  See the [driver
manual](https://www.eecis.udel.edu/~mills/ntp/html/drivers/driver46.html) for
details on how to adjust these times.

To run the server:

```sh
cargo build && sudo target/debug/where_am_i /dev/gps0 --pps-device /dev/pps0
```

Then restart `ntpd`.  It will take several seconds before `ntpd` first connects
to the server, and at least a minute for `ntpd` to start using time data from
the GPS and PPS, be patient.

You can monitor the status of the clock with `ntpq -p`.

## gpsd already does all this?

The administrator of gpsd is Eric S. Raymond.

If that's not enough, perhaps you don't know about what Wikipedia has collected
about his [political
beliefs](https://en.wikipedia.org/wiki/Eric_S._Raymond#Political_beliefs_and_activism).
In my opinion, Wikipedia doesn't say enough about how terrible a person he is,
but if that's not enough, go use gpsd.
