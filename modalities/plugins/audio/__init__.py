
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
# TODO: Implement resource contention resolution (accounting for audio usage)
#   Look into adding a "wake word" for these situations
# TODO: Add in spotify playback (once the web api allows it)
#   Look at alternate approaches to music streaming
# TODO: Implement voice recognition (probably requires AI)


# NOTE: This plugin is designed to handle both audio playback through device speakers, etc. and voice recognition dispatch through device microphones
# However, some devices may not have any microphones to listen to, making some chunk of this plugin unusable
# As currently implemented, the plugin "ignores" any voice recognition code if it is unable to setup the device's microphone
# It may be beneficial, to instead split this plugin into two plugins, each handling only playback or voice recognition, but not both
# I have chosen for the moment to not pursue this approach as the voice recognition currently uses the playback system to indicate "listening" status
# NOTE: The voice recognition system does not use a "wake" keyword. It is always on though that is something to work on to improve system performance
class AudioPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super().__init__(logger, config=config)
        self._role = 'audio'

        self._speaker = pyaudio.PyAudio()
        self._voice = win32com.client.Dispatch('SAPI.SpVoice')

        # Handle errors in microphone setup (indicating the device does not have a usable microphone)
        try:
            self._mic = sr.Microphone()
            self._rec = sr.Recognizer()
            with self._mic as source:
                self._rec.adjust_for_ambient_noise(source)
            self._mic_initialized = True
            self._log.info("Successfully setup device microphone. Voice recognition capabilities enabled")

        except Exception as e:
            self._log.error("Couldn't setup device microphone: {}".format(e))
            self._log.error("  Voice recognition capabilities disabled")
            self._mic_initialized = False

        self._audio_control = asyncio.Lock()
        self._played_beep = False

        # Register the audio playback handles
        self._register_handle('play', AudioPlugin.handle_play)
        self._register_handle('speak', AudioPlugin.handle_speak)

    async def run(self, comm):
        if not self._mic_initialized:
            return True

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
                self._log.error("Microphone triggered but was unable to recognize audio snippet")

            else:
                self._log.debug("HEARD <{}>".format(query))

                msg = Message(plugin=self)
                msg.action = 'dispatch'
                msg.args = query
                comm.send(msg, self._log)

        return True

    async def handle_play(self, msg, comm):
        if len(msg.args) == 0:
            self._log.error("Received play request with no song-or-music data. msg.id={}".format(msg.id))
            return

        song = msg.args[0]
        if not os.path.exists(song):
            self._log.debug("The requested song `{}` does not exist on the system. Assuming it is a search query instead".format(song))

            song_path_request = Message(plugin=self)
            song_path_request.action = 'search'
            song_path_request.args = song
            song_path_request.send_to(role='manager')

            resp = await comm.wait_for_response(song_path_request, self._log)

            if len(resp.resp) == 0:
                self._log.error("Search results were empty. Song does not exist within the system".format(song))
                return

            song = resp.resp[0]
            if not os.path.exists(song):
                self._log.error("Song file does not exist at returned path `{}`. Cannot play song as requested".format(song))
                return

        with await self._audio_control:
            self._play_song(song)
            self._played_beep = False

    async def handle_speak(self, msg, comm):
        with await self._audio_control:
            self._voice.Speak(msg.args[0])

    def _play_song(self, song):
        self._log.info("Playing requested music file at `{}`".format(song))

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
