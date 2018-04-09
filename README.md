# throttle cpu temp

This Linux application collects the temperature of the processor, and if
the temperature gets higher than a configured maximum, it starts to
throttle down the processor frequency to reduce the temperature.

It was created to cope with high processor temperature on laptops while
doing large compilations (ex: Linux kernel).

The idea and logic were based on `https://github.com/Sepero/temp-throttle`,
a shell script. This application was developed in Rust and is intended to
be executed as a daemon, using little system resources.

# Installing

You will need the Rust compiler installed on your system.

Compile and install:

```bash
make
make install
```

# Warnings

This application works for me, but there is no warranty! It could explode
your computer and burn down your house. Use at your own risk! You were
warned!

Please refer to `COPYING` for more licensing details.
