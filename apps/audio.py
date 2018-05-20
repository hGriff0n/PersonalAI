#!/usr/bin/env python3

import asyncio

# RPC communication (may not be desirable: want to send message to a client other than the "caller")
import Pyro4

# Speech Recognition / TTS
import speech_recognition as sr
# from tts.sapi import Sapi as Client     # Why is this taking a long time
import win32com.client

# Music/general audio
import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks

from common import logger
import os

# Long term dev work
# TODO: Improve interactivity of AI
#   For instance, say what "song" is playing when I want to play music
#   Work on modifying the beep tone to be more "pleasant" (its too loud for one)
#   Work on making the voice a bit louder (I can't hear it)
# TODO: Look at replacing asyncio with Trio
# TODO: Implement a database (or something) to track all local music files
#   This would end up being subsumed by the "backing storage" server though (it's the responsibility)
#     NOTE: This may be handled by server not dispatching "play" events to the cli app (in which case I need to rework the control flow of this app)
#   I'll probably have to implement a queuing/thread system to handle networked requests
# TODO: Implement resource contention resolution (accounting for audio usage)
#   Cli app should not have to wait for a song to finish playing to interact, audio does have to wait
#   Look into adding a "wake word" for these situations
# TODO: Add in spotify playback (once the web api allows it)
#   Look at alternate approaches to music streaming
# TODO: Implement voice recognition (probably requires AI)

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

audio = {
    'mic': sr.Microphone(),
    'voice': win32com.client.Dispatch("SAPI.SpVoice"),
    'speaker': pyaudio.PyAudio()
}

# Temporary database for initial testing purposes
songs = {
    'Magnet': r"C:\Users\ghoop\Desktop\PersonalAI\data\Magnet.mp3",
    'Living on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Livin on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Aerosmith': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Anstatt Blumen': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
}


# Temporary wrapper to enable playing a song
def play_song(song):
    _, ext = os.path.splitext(song)
    seg = AudioSegment.from_file(song, ext[1:])

    p = audio['speaker']
    stream = p.open(format=p.get_format_from_width(seg.sample_width),
                    channels=seg.channels,
                    rate=seg.frame_rate,
                    output=True)

    # break audio into half-second chunks (to allows keyboard interrupts)
    for chunk in make_chunks(seg, 500):
        stream.write(chunk._data)

    stream.stop_stream()
    stream.close()




# Main event function which handles input and dispatching
async def run():
    rec = sr.Recognizer()

    with audio['mic'] as source:
        rec.adjust_for_ambient_noise(source)
        play_song("data\\low_beep.mp3")
        audio_data = rec.listen(source)

    try:
        query = rec.recognize_google(audio_data)
        log.info("HEARD <{}>".format(query))

        answer = dispatcher.dispatch(query)
        log.info("DISPATCHED <{}>".format(answer))

        # NOTE: "Stop" answer isn't matched to the input medium
        audio['voice'].Speak(answer['text'])
        if not answer['stop']:
            asyncio.ensure_future(run())

    except sr.UnknownValueError:
        asyncio.ensure_future(run())                            # Since we never entered dispatch, we still need to run
        log.error("Couldn't recognize audio")

    except Exception as e:
        asyncio.ensure_future(run())
        log.error(e)



# TODO: Make this event process concurrent and distributed
if __name__ == "__main__":
    dispatcher = Pyro4.Proxy("PYRONAME:ai.dispatch")
    asyncio.ensure_future(run())

    # Run until no more functions are scheduled
    while True:
        log.info("Gathering tasks")
        pending_tasks = [task for task in asyncio.Task.all_tasks() if not task.done()]
        if len(pending_tasks) == 0: break
        asyncio.get_event_loop().run_until_complete(asyncio.gather(*pending_tasks))

# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   tts.SAPI: https://github.com/DeepHorizons/tts
#   Wit: https://wit.ai/docs
#   Pydub: https://github.com/jiaaro/pydub
