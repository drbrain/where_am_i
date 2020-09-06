# Where am I?

## What is this?

This project aims to replace enough of gpsd to allow me to feed timestamp and
PPS data into the [Network Time Protocol](https://www.ntp.org) daemon, and to
allow tracking of GPS statistics.

Presently it is only known to work with [u-blox
ZED-F9P](https://www.u-blox.com/en/product/zed-f9p-module)-based GNSS
receivers.

I am also working with a GlobalTop-based GNSS receiver for expanded
compatibility.

## How do I use this?

### Raspberry Pi configuration

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

### NTP shared memory driver

The [shared memory driver](http://doc.ntp.org/4.2.8/drivers/driver28.html) can
be used with units 2 (GPS data) and 3 (PPS data) by adding the driver to
`/etc/ntp.conf`:

```
server 127.127.28.2 mode 1
fudge 127.127.28.2 refid GPS

server 127.127.28.3 mode 1 prefer
fudge 127.127.28.3 refid PPS
```

### NTP GPSD JSON driver

The [GPSD_JSON
refclock](http://doc.ntp.org/4.2.8/drivers/driver46.html) requires `/dev/gps0`
to be a symlink to the GPS UART, so I used `udev` to create it by adding a
`/etc/udev/rules.d/50-gps.rules`:

```
KERNEL=="ttyAMA0", SYMLINK+="gps0"
```

`/dev/ttyAMA0` is the default name of the hardware UART.  After restarting the
`udev` service you should see `/dev/gps0` as a symlink to `/dev/ttyAMA0`.

Presently only enough of the gpsd protocol to support the GPSD_JSON refclock is
implemented.  This includes the `?VERSION` command and enough of the `?WATCH`
commands for ntpd to stream `TOFF` and `PPS` events.  (The refclock driver
manual page says it requires `TPV` events, but if the gpsd protocol version is
3.10 it only reads `TOFF` and `PPS` events depending.)

Add the driver to `/etc/ntp.conf` with:

```
server 127.127.46.0
fudge 127.127.46.0
```

### NTP driver notes

After running NTP for a while and getting stable output you will need to adjust
the NTP offsets for each driver.  This is done through the `time1` fudge
setting for the NTP SHM driver and the `time` and `time2` fudge settings for
the GPSD_JSON driver.

See the respective driver manual pages for more info.

### Running where_am_i

This sections shows how to set up `where_am_i` for timekeeping purposes.
Positioning is not yet implemented.

First create a `where.toml`:

```toml
[[gps]]
name = "GPS0"
device = "/dev/gps0"
baud_rate = 38400
messages = [ "ZDA" ]
ntp_unit = 2

[gps.pps]
device = "/dev/pps0"
ntp_unit = 3
```

This will send GNSS timestamps to NTP SHM unit 2 and PPS timestamps to NTP SHM
unit 3.  You can choose other NTP SHM units if you like.

To run the server:

```sh
cargo build && sudo target/debug/where_am_i where.toml
```

Then restart `ntpd`.  It will take several seconds before `ntpd` first connects
to the gpsd server, and at least a minute for `ntpd` to start using time data from
the GPS and PPS, be patient.

You can monitor the status of the clock with `ntpq -p`.

## where.toml

### Global options

### `[[gps]]` options

The `[[gps]]` section may be repeated if you have more than one GPS receiver.
It supports the following fields:

* `name`: A friendly name for the GPS device.
* `device`: The TTY device to open to interact with the GPS.
* `baud_rate`: The speed of the GPS device, defaults to 38400.
* `framing`: The data bits, parity bit, and stop bit configuration.  Defaults to `"8N1"`.
* `flow_control`: GPS device flow control.  Defaults to none.  Maybe be empty.
  (none), `"H"` for hardware flow control or `"S"` for software flow control.
* `timeout`: Timeout for reading from the GPS device in milliseconds.  Defaults to 1 ms.
* `messages`: List of messages to enable for a u-blox GPS device.  Defaults to all known.
* `ntp_unit`: NTP SHM unit to use for sending timestamps.  Defaults to none.

### `[gps.pps]` options

The `[gps.pps]` section allows you to attach a PPS device to a GPS device.

* `device`: The name of the PPS device to open.
* `ntp_unit`: NTP SHM unit to use for sending timestamps.  Defaults to none.

## gpsd already does all this?

The administrator of gpsd is Eric S. Raymond.

If that's not enough, perhaps you don't know about what Wikipedia has collected
about his [political
beliefs](https://en.wikipedia.org/wiki/Eric_S._Raymond#Political_beliefs_and_activism).
In my opinion, Wikipedia doesn't say enough about how terrible a person he is,
but if that's not enough, go use gpsd.
