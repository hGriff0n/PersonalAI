
import asyncio
import os

# STT / TTS
import speech_recognition as sr
# from tts.sapi import Sapi as Client     # Why is this taking a long time
import win32com.client

# Music/general audio
import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks

from common.msg import Message
from common import plugins

# Long term dev work
# TODO: Improve interactivity of AI
#   Work on modifying the beep tone to be more "pleasant" (its too loud for one)
#   Work on making the voice a bit louder (I can't hear it)
#   Move resetting the 'play_beep' code to the dispatch app
#     We can still accept input before then, but this should eliminate some annoyance with using the plugin to play music
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

class AudioPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super.__init__(self, logger, config=config)

        self._speaker = pyaudio.PyAudio()
        self._voice = win32com.client.Dispatch('SAPI.SpVoice')

        self._mic = sr.Microphone()
        self._rec = sr.Recognizer()
        with self._mic as source:
            self._rec.adjust_for_ambient_noise(source)

        self._audio_control = asyncio.Lock()
        self._played_beep = False

        self._register_handle('play', AudioPlugin.handle_play)
        self._register_handle('speak', AudioPlugin.handle_speak)

    async def run(self, comm):
        with self._mic as source:
            try:
                audio = None

                with await self._audio_control:
                    if not self._played_beep:
                        self._play_song("data\\low_beep.mp3")
                        self._played_beep = True

                    audio = self._rec.listen(source, 0.4, None)
                query = self._rec.recognize_google(audio)

            except sr.WaitTimeoutError:
                pass

            except sr.UnknownValueError:
                self._log.error("Couldn't recognize audio")

            else:
                self._log.debug("HEARD <{}>".format(query))
                msg = Message(plugin=self, role='audio')
                msg.action = 'dispatch'
                msg.args = query
                comm.send(msg)

        return True

    async def handle_play(self, msg, comm):
        with await self._audio_control:
            if len(msg.args) > 1:
                self._voice.Speak(msg.args[1])

            self._log.debug("Playing <{}>".format(msg.args[0]))
            song = songs.get(msg.args[0], msg.args[0])
            self._play_song(song)
            self._played_beep = False

    async def handle_speak(self, msg, comm):
        with await self._audio_control:
            self._voice.Speak(msg.args[0])

    def _play_song(self, song):
        _, ext = os.path.splitext(song)
        seg = AudioSegment.from_file(song, ext[1:])

        p = self._speaker
        stream = p.open(
            format=p.get_format_from_width(seg.sample_width),
            channels=seg.channels,
            rate=seg.frame_rate,
            output=True
        )

        # Split audio into half-second chunks to allow for interrupts
            # Is this actually allowing for interrupts?
        for chunk in make_chunks(seg, 300):
            stream.write(chunk._data)

        stream.stop_stream()
        stream.close()
