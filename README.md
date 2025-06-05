# Extract software update packages for Stern pinball machines

[Stern pinball machines](https://www.sternpinball.com/) based on Spike 2 have
their software updates packaged as .spk files. These are downloaded over the
air by Internet-connected machines or may be manually installed by the owner by
copying them to a flash drive that is then inserted into a USB slot on the
MPU's motherboard.

This tool can parse, validate, and extract these update packages. It supports both
single file and split update formats.

# Usage

Given an update file or a directory containing split update files:

```
$ ls -lh ~/Downloads/jurassic_park_le-1_15_0.spk
total 5653080
-rw-rw-r--@ 1 mrowe  staff   1.9G Nov  7  2024 jurassic_park_le-1_15_0.spk.002.000
-rw-rw-r--@ 1 mrowe  staff   840M Nov  7  2024 jurassic_park_le-1_15_0.spk.002.001
```

Verify the contents of the update:

```
$ stern-spk verify ~/Downloads/jurassic_park_le-1_15_0.spk
Package: spike
Version: 2.7.0
/bin/chattr.e2fsprogs                                        mode=100755 size=       7820  md5: ✔  hmac: ✔
/etc/ca-certificates.conf                                    mode=100644 size=       7609  md5: ✔  hmac: ✔
/etc/fb.modes                                                mode=100755 size=        208  md5: ✔  hmac: ✔
/etc/fstab                                                   mode=100644 size=       1925  md5: ✔  hmac: ✔
/etc/init.d/alignment.sh                                     mode=100755 size=        250  md5: ✔  hmac: ✔
/etc/init.d/alsa-state                                       mode=100755 size=        811  md5: ✔  hmac: ✔
[…]
/games/jurassic_park_le/tmc2590node-LPC1313-1_19_0.hex       mode=100664 size=      34692  md5: ✔  hmac: ✔
/games/jurassic_park_le/tmc5041node-LPC1313-1_19_0.hex       mode=100664 size=      40460  md5: ✔  hmac: ✔
/games/jurassic_park_le/ws2812node-LPC1313-1_19_0.hex        mode=100664 size=      40460  md5: ✔  hmac: ✔
```

Extract the files from the update:

```
$ stern-spk extract ~/Downloads/jurassic_park_le-1_15_0.spk
Verifying contents of file... done!


Extracting package spike to /Users/mrowe/Downloads/jurassic_park_le-1_15_0/spike
   usr/bin/lsattr
   usr/bin/mk_cmds
   usr/bin/compile_et
[…]


Extracting package jurassic_park_le to /Users/mrowe/Downloads/jurassic_park_le-1_15_0/jurassic_park_le
   jurassic_park_le/coil4node-LPC1112_101-1_19_0.hex
   jurassic_park_le/coil4node-LPC1112_201-1_19_0.hex
   jurassic_park_le/coil4node-LPC1313-1_19_0.hex
[…]
```

Verification takes 5-10 seconds, depending on the size of the update file.
Extraction takes a few seconds longer since it verifies the files before writing
them to disk.

## Content-addressable storage

When extracting multiple updates, use `--cas-dir` to enable deduplication:

```
$ stern-spk extract ~/Downloads/jurassic_park_le-1_15_0.spk --cas-dir ~/spk-archive
```

This stores files by content hash and uses reflinks/hardlinks to avoid duplicates,
significantly reducing disk usage when extracting multiple updates of the same game.

# The format

## Single file update format

The format consists of chunks with 8 byte headers like so:

```rust
enum ChunkSize {
   #[br(magic = 0xffff_ffffu32)]
   New(u64),
   Old(u32),
}

struct Header {
   magic: [u8; 4],
   size: ChunkSize,
}
```

The magic value indicates the type of the chunk (`SPKS`, `SPK0`, `SIDX`,
`STRS`, `FINF`, `FI64`, `FEND`, `SDAT`, `SZ64`). The size indicates the number
of bytes contained within the chunk. Note that the size may be either `ffff ffff`
followed by a 64-bit size, or a 32-bit value. The `ffff ffff` is for
backward-compatibility and presumably prevents older software that assumes
32-bit sizes are used from misinterpreting the file.

Chunks can contain other chunks.

`SPKS` is the top-level chunk. Its header contains the number of `SPK0` chunks
contained within. Each `SPK0` chunk corresponds to a partition on the system
that will be updated.

`SPK0` chunks serve as containers for `SIDX`, `STRS`, `FINF` and `SDAT` chunks
and do not contain any additional data themselves.

`SIDX` provides metadata, including the partition type to updated, the package
name, and its version number. 

`STRS` contains null-separated strings for all of the file names contained
within the package.

After `STRS` are zero or more `FINF` / `FI64` chunks containing information
aboout the files contained within the update. These are terminated by a `FEND`
chunk.

`FINF` / `FI64` chunks contain the file name (represented as an offset into `STRS`), the
file size, the offset of the file data within `SDAT`, an HMAC-SHA1 of the data,
and an MD5 of the data.

`FINF` uses 32-bit fields for sizes and offsets, while `FI64` uses 64-bit fields.

`SDAT` chunks contain the file data. The data is indexed by `FINF`.

It is unknown what purpose `SZ64` serves at this time.


## Split update format

Since updates are installed on machines by being copied onto USB drives, the
file size limitations of FAT32 drives are a concern. Updates that may come
close to those limits are split into multiple smaller files.

Rather than simply splitting the .spk file into multiple files, they are first
encapsulated in a [SquashFS file
system](https://tldp.org/HOWTO/SquashFS-HOWTO/whatis.html). These are then
split.

The Spike 2 system software uses `affuse` to present the chunks of the SquashFS
file as a single logical file that is in then mounted via SquashFS.

# Additional information

The software update packages only include the files that have changed since the
original release. This means many files from underlying components such as
system libraries and the Linux kernel are not present. If you need these, look
to the [SD card images that Stern makes
available](https://sternpinball.com/support/sd-cards/).
