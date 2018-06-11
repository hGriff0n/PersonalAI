
import os
import threading
import traceback

from common import logger
from plugins import Plugin

# STT / TTS
import speech_recognition as sr
# from tts.sapi import Sapi as Client     # Why is this taking a long time
import win32com.client

# Music/general audio
import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks

# TODO: Incorporate ideas from 'altio.py' to enable correct usage


# Temporary database for initial testing purposes
songs = {
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

        self.log = logger.create('audio.log')
        self.log.setLevel(logger.logging.INFO)

    def run(self, queue):
        rec = sr.Recognizer()

        try:
            while True:
                with self.mic as source:
                    rec.adjust_for_ambient_noise(source)
                    self._play_song("data\\low_beep.mp3")
                    audio_data = rec.listen(source)

                try:
                    query = rec.recognize_google(audio_data)
                    self.log.info("HEARD <{}>".format(query))

                    queue.put({ 'heard': query })
                except sr.UnknownValueError:
                    self.log.error("Couldn't recognize audio")

        except Exception:
            self.log.error("EXCEPTION: " + traceback.format_exc())
            queue.put("quit")

    def dispatch(self, msg, queue):
        if 'play' in msg:
            self._play_song(msg['play'])

        elif 'text' in msg:
            self.voice.Speak(msg['text'])

        return 'stop' not in msg

    def _play_song(self, song):
        _, ext = os.path.splitext(song)
        seg = AudioSegment.from_file(song, ext[1:])

        p = self.speaker
        stream = p.open(format=p.get_format_from_width(seg.sample_width),
                        channel=seg.channels,
                        rate=seg.frame_rate,
                        output=True)

        # Split audio into half-second chunks to allow for interrupts
        for chunk in make_chunks(seg, 500):
            stream.write(chunk._data)

        stream.stop_stream()
        stream.close()

# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   tts.SAPI: https://github.com/DeepHorizons/tts
#   Pydub: https://github.com/jiaaro/pydub
