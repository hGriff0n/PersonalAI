#!/usr/bin/env python3

import asyncio

# AI recognition
from wit import Wit

# Speech Recognition / TTS
import speech_recognition as sr
from tts.sapi import Sapi as Client

# Music/general audio
import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks

from common import logger
import os

# Immediate development work
# TODO: Improve intent extraction/dispatch
  # Implement the ability to play songs through spotify
    # This may be difficult, maybe hardcode in some songs to play
# TODO: Add in broader control over the computer's audio systems
  # Change the "I'm listening" signal to a small beep
# NOTE: After this work is done, shift over to cli app

# Long term dev work
# TODO: Implement a database (or something) to track all local music files
#   This would end up being subsumed by the "backing storage" server though (it's the responsibility)
# TODO: Implement resource contention resolution (accounting for audio usage)
#   Look into adding a "wake word" for these situations
# TODO: Add in spotify playback (once the web api allows it)
#   Look at alternate approaches to music streaming
# TODO: Implement voice recognition (probably requires AI)

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)

audio = {
    'mic': sr.Microphone(),
    'voice': Client(),
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
    seg = AudioSegment.from_file(song, ext)

    p = audio['speaker']
    stream = .open(format=p.get_format_from_width(seg.sample_width),
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
        print("> ...")
        audio_data = rec.listen(source)

    try:
        query = rec.recognize_google(audio_data)
        log.info("HEARD <{}>".format(query))
        dispatch(query, audio['voice'])

    except sr.UnknownValueError:
        asyncio.ensure_future(run())                            # Since we never entered dispatch, we still need to run
        log.error("Couldn't recognize audio")

    except Exception as e:
        asyncio.ensure_future(run())
        log.error(e)



# Pass along the speech data to determine what to do
def dispatch(query, voice):
    msg = client.message(query)
    log.info("MSG <{}>".format(str(msg)))

    action = msg['entities']
    schedule_run = True

    # Perform intent recognition and dispatch
    if 'intent' in action:
        log.info("INTENTS <{}>".format(action['intent']))

        intent = action['intent'][0]        # TODO: Figure out why 'intent' is an array
        schedule_run = 'stop' != intent['value']

        voice.say("You said \"{}\"".format(query))

    elif 'greetings' in action:
        log.info("GREETING")
        voice.say("Hello")

    elif 'thanks' in action:
        log.info("THANKS")
        voice.say("Thanks")

    elif 'bye' in action:
        log.info("GOODBYE")
        voice.say("Bye")
        schedule_run = False

    else:
        log.info("Unknown Action")
        voice.say("I have no idea what you're saying")

    # Schedule another run unless we need to stop
    if schedule_run:
        asyncio.ensure_future(run())



# TODO: Make this event process concurrent and distributed
if __name__ == "__main__":
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
