#!/usr/bin/env python3

import asyncio

import Pyro4

from wit import Wit

from common import logger

import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks
import os

# Immediate development work
# TODO: Enable utilizing resources across apps
#   Extend the communication system to allow for dispatch to send a signal to an app other than the caller
#     (ie. allow for the cli app to trigger song playing in the audio app)
#   Will need to modify the code framework for these apps to account for this inversion
#     Maybe register the clients in the nameserver as well
# TODO: Develop app to handle dispatch, forward action to that app
  #   The scripts pyro4-check-config.exe, pyro4-flameserver.exe, pyro4-httpgateway.exe, pyro4-ns.exe, pyro4-nsc.exe and pyro4-test-echoserver.exe are installed in 'C:\Users\ghoop\AppData\Roaming\Python\Python36\Scripts' which is not on PATH.
  #   Consider adding this directory to PATH or, if you prefer to suppress this warning, use --no-warn-script-location.
# TODO: Develop system to enable asynchronous working with non-colliding resources
#   I should still be able to type and interact with the cli app while music is playing (not for audio)
#   NOTE: This may be handled by the "server" just not handling that within the cli app (still need to handle audio restrictions)

log = logger.create('dispatch.log')
log.setLevel(logger.logging.INFO)


songs = {
    'Magnet': r"C:\Users\ghoop\Desktop\PersonalAI\data\Magnet.mp3",
    'Living on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Livin on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Aerosmith': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Anstatt Blumen': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
}


@Pyro4.expose
@Pyro4.behavior(instance_mode="single")
class Dispatcher:
    def __init__(self):
        self.client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)
        self.speaker = pyaudio.PyAudio()

    def dispatch(self, query):
        msg = self.client.message(query)
        log.info("MSG <{}>".format(str(msg)))
        action = msg['entities']
        answer = { 'text': '', 'stop': False }

        if 'intent' in action:
            log.info("INTENTS <{}>".format(action['intent']))

            intent = action['intent'][0]        # TODO: Figure out why 'intent' is an array
            answer['stop'] = 'stop' == intent['value']

            if intent['value'] == "play_music":
                song = None

                if 'search_query' in action:
                    title = action['search_query'][0]['value']

                    if title in songs:
                        song = songs[title]

                if song is None:
                    song = songs['Magnet']

                log.info("PLAYING <{}>".format(song))
                answer['text'] = "Playing {}".format(song)
                play_song(self.speaker, song)

            else:
                answer['text'] = "You typed \"{}\"".format(query)

        elif 'greetings' in action:
            log.info("GREETING")
            answer['text'] = "Hello"

        elif 'thanks' in action:
            log.info("THANKS")
            answer['text'] = "Thanks"

        elif 'bye' in action:
            log.info("GOODBYE")
            answer['text'] = "Bye"
            answer['stop'] = True

        else:
            log.info("Unknown Action")
            answer['text'] = "I have no idea what you are doing"

        return answer


# Temporary wrapper to enable playing a song
def play_song(speaker, song):
    _, ext = os.path.splitext(song)
    seg = AudioSegment.from_file(song, ext[1:])

    stream = speaker.open(format=speaker.get_format_from_width(seg.sample_width),
                    channels=seg.channels,
                    rate=seg.frame_rate,
                    output=True)

    # break audio into half-second chunks (to allows keyboard interrupts)
    for chunk in make_chunks(seg, 500):
        stream.write(chunk._data)

    stream.stop_stream()
    stream.close()



if __name__ == "__main__":
    log.info("Initializing daemon")
    # TODO: Look at `Pyro4.naming.startNS() and Pyro4.naming.startNSloop()`
    Pyro4.Daemon.serveSimple({ Dispatcher: "ai.dispatch" }, ns=True)


# API Documentation:
#   Wit: https://wit.ai/docs
