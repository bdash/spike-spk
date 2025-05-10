# Parse and verify .spk files used by software updates for Stern pinball machines

[Stern pinball machines](https://www.sternpinball.com/) based on Spike 2 have
their software updates packaged as .spk files. These are downloaded over the
air by Internet-connected machines or may be manually installed by the owner by
copying them to a flash drive that is then inserted into a USB slot on the
MPU's motherboard.

This tool can parse and validate the contents of these files. It supports both single file and split update formats.

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

# TODO

Extract files to disk. The data is right there, but care needs to be taken when
creating the files on disk. It's easier to get the file data from the [SD card
images that Stern makes available](https://sternpinball.com/support/sd-cards/).

