#!/usr/bin/env python3
"""Send the normal X11 WM_DELETE_WINDOW request without requiring a window manager."""
import ctypes as c,os,subprocess
assert os.environ.get('DISPLAY') not in [None,':0'],'Use an isolated test display'
x=c.CDLL('libX11.so.6')
x.XOpenDisplay.argtypes=[c.c_char_p];x.XOpenDisplay.restype=c.c_void_p
x.XInternAtom.argtypes=[c.c_void_p,c.c_char_p,c.c_int];x.XInternAtom.restype=c.c_ulong
class Data(c.Union):_fields_=[('b',c.c_char*20),('s',c.c_short*10),('l',c.c_long*5)]
class Client(c.Structure):_fields_=[('type',c.c_int),('serial',c.c_ulong),('send_event',c.c_int),('display',c.c_void_p),('window',c.c_ulong),('message_type',c.c_ulong),('format',c.c_int),('data',Data)]
class Event(c.Union):_fields_=[('client',Client),('pad',c.c_long*24)]
x.XSendEvent.argtypes=[c.c_void_p,c.c_ulong,c.c_int,c.c_long,c.POINTER(Event)]
x.XFlush.argtypes=[c.c_void_p];x.XCloseDisplay.argtypes=[c.c_void_p]
d=x.XOpenDisplay(None);assert d,'Cannot connect to test display'
try:
 windows=subprocess.check_output([os.environ.get('XDOTOOL','xdotool'),'search','--name','^OpenPDF Desktop$'],text=True).split()
 assert len(windows)==1,'Expected one test application'
 w=int(windows[0]);e=Event();e.client.type=33;e.client.display=d;e.client.window=w;e.client.message_type=x.XInternAtom(d,b'WM_PROTOCOLS',0);e.client.format=32;e.client.data.l[0]=x.XInternAtom(d,b'WM_DELETE_WINDOW',0)
 assert x.XSendEvent(d,w,0,0,c.byref(e));x.XFlush(d)
finally:x.XCloseDisplay(d)
