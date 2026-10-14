#!/usr/bin/env python3
"""Generate this project's original document icon; no third-party artwork."""
import pathlib,struct,zlib
root=pathlib.Path(__file__).resolve().parents[1];n=256;pixels=bytearray()
for y in range(n):
    pixels.append(0)
    for x in range(n):
        c=(38,100,210,255) if 12<=x<244 and 12<=y<244 else (0,0,0,0)
        if 64<=x<192 and 40<=y<216 and not(x>152 and y<80 and x-152>y-40):c=(255,255,255,255)
        if 82<=x<170 and any(t<=y<t+8 for t in (106,130,154,178)):c=(38,100,210,255)
        pixels.extend(c)
def chunk(t,b):return struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b)&0xffffffff)
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',n,n,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(pixels))+chunk(b'IEND',b'')
(root/'assets/icon.png').write_bytes(png)
(root/'assets/icon.ico').write_bytes(struct.pack('<HHH',0,1,1)+struct.pack('<BBBBHHII',0,0,0,0,1,32,len(png),22)+png)
(root/'assets/icon.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256"><rect x="12" y="12" width="232" height="232" rx="32" fill="#2664d2"/><path d="M64 40h88l40 40v136H64z" fill="white"/><path d="M82 110h88m-88 24h88m-88 24h88m-88 24h88" stroke="#2664d2" stroke-width="8"/></svg>')
