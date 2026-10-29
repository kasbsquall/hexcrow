"""Drop every WASM custom section except `tari_tdef`.

The Ootle engine rejects a published binary that still carries rustc's `name`, `producers` or
`target_features` sections; the wallet daemon normally removes them with wasm-opt.
Usage: python tools/strip_wasm.py in.wasm out.wasm
"""
import sys


def read_uleb(buf, i):
    result = shift = 0
    while True:
        b = buf[i]
        i += 1
        result |= (b & 0x7F) << shift
        if b < 0x80:
            return result, i
        shift += 7


def main(src, dst):
    data = open(src, "rb").read()
    assert data[:8] == b"\x00asm\x01\x00\x00\x00", "not a wasm module"
    out, i, dropped = bytearray(data[:8]), 8, []
    while i < len(data):
        start = i
        section_id = data[i]
        size, body = read_uleb(data, i + 1)
        end = body + size
        if section_id == 0:
            name_len, n = read_uleb(data, body)
            name = data[n:n + name_len].decode()
            if name != "tari_tdef":
                dropped.append(name)
                i = end
                continue
        out += data[start:end]
        i = end
    open(dst, "wb").write(out)
    print(f"{len(data)} -> {len(out)} bytes, dropped custom sections: {dropped}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
