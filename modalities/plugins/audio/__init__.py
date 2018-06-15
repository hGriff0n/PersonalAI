#!/usr/bin/env python3

import os
import threading

from common import logger
from common.msg import Message
from plugins import Plugin

# STT / TTS
import speech_recognition as sr
# from tts.sapi import Sapi as Client     # Why is this taking a long time
import win32com.client

# Music/general audio
import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks

# Short term dev work
# TODO: Improve shutdown when the server stops ("run" doesn't quit)
    # Can't quit out of running audio sample

# Long term dev work
# TODO: Improve interactivity of AI
#   Work on modifying the beep tone to be more "pleasant" (its too loud for one)
#   Work on making the voice a bit louder (I can't hear it)
# TODO: Implement a database (or something) to track all local music files
#   This would end up being subsumed by the "backing storage" server though (it's the responsibility)
#     NOTE: This may be handled by server not dispatching "play" events to the cli app (in which case I need to rework the control flow of this app)
#   I'll probably have to implement a queuing/thread system to handle networked requests
# TODO: Implement resource contention resolution (accounting for audio usage)
#   Look into adding a "wake word" for these situations
# TODO: Add in spotify playback (once the web api allows it)
#   Look at alternate approaches to music streaming
# TODO: Implement voice recognition (probably requires AI)


# Temporary database for initial testing purposes
songs = {
    'music': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Magnet': r"C:\Users\ghoop\Desktop\PersonalAI\data\Magnet.mp3",
    'Living on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Livin on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Aerosmith': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Anstatt Blumen': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
}

class AudioPlugin(Plugin):
    def __init__(self):
        self.speaker = pyaudio.PyAudio()
        self.mic = sr.Microphone()
        self.voice = win32com.client.Dispatch('SAPI.SpVoice')
        self.audio_control = threading.Lock()

        self.log = logger.create('audio.log')
        self.log.setLevel(logger.logging.INFO)


    def run(self, queue):
        rec = sr.Recognizer()
        with self.audio_control:
            with self.mic as source:
                rec.adjust_for_ambient_noise(source)

        played_beep = False

        with self.mic as source:
            while True:
                try:
                    audio = None

                    with self.audio_control:
                        if not played_beep:
                            self._play_song("data\\low_beep.mp3")
                            played_beep = True

                        audio = rec.listen(source, 1, None)

                    query = rec.recognize_google(audio)

                except sr.WaitTimeoutError:
                    continue

                except sr.UnknownValueError:
                    self.log.error("Couldn't recognize audio")
                    played_beep = False
                    continue

                except Exception:
                    raise

                else:
                    self.log.info("HEARD <{}>".format(query))
                    self.send_message(query, queue)
                    played_beep = False


    def send_message(self, query, queue):
        msg = Message('audio')
        msg.dispatch(query)
        queue.put(msg)


    def dispatch(self, msg, queue):
        if 'action' in msg:
            if msg['action'] == 'play':
                with self.audio_control:
                    if 'text' in msg:
                        self.voice.Speak(msg['text'])
                    self._play_song(songs[msg['play']])

        elif 'text' in msg:
            with self.audio_control:
                self.voice.Speak(msg['text'])

        # if msg['stop']:
        #     queue.put("quit")


    def _play_song(self, song):
        _, ext = os.path.splitext(song)
        seg = AudioSegment.from_file(song, ext[1:])

        p = self.speaker
        stream = p.open(format=p.get_format_from_width(seg.sample_width),
                        channels=seg.channels,
                        rate=seg.frame_rate,
                        output=True)

        # Split audio into half-second chunks to allow for interrupts
        for chunk in make_chunks(seg, 500):
            stream.write(chunk._data)

        stream.stop_stream()
        stream.close()

    def get_hooks(self):
        return [ 'audio' ]

# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   tts.SAPI: https://github.com/DeepHorizons/tts
#   Pydub: https://github.com/jiaaro/pydub
