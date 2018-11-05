
import asyncio

from wit import Wit

from common.msg import Message
from common import plugins

# NOTE: This app is wholely responsible for receiving a 'msg' json object from the server and running it through
    # wit.ai to determine what the message is wanting to perform in the context of the system
# TODO: Improve code quality and naming

class DispatchPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super().__init__(logger, config=config)

        self._role = 'dispatch'
        self._client = Wit(config['wit_token'], logger=logger)
        self._register_handle('dispatch', DispatchPlugin.handle_dispatch)

        self._intent_handles = {
            'stop': DispatchPlugin._handle_stop,
            'play_music': DispatchPlugin._handle_music,
            'find': DispatchPlugin._handle_find
        }

    def _validate_configuration(self, config):
        if 'wit_token' not in config:
            raise Exception("Missing wit.ai api token")

    async def run(self, comm):
        return True

    async def handle_dispatch(self, msg, comm):
        dispatch = msg.args[0]
        self._log.info("Received dispatch message: {}".format(dispatch))

        # TODO: Can we make the wit.ai call asynchronous?
        query = self._client.message(dispatch)
        quest = query['entities']

        if 'intent' in quest:
            self._log.info("INTENT <{}>".format(quest['intent']))
            intent = quest['intent'][0]['value']

            if intent in self._intent_handles:
                await self._intent_handles[intent](self, msg, comm, quest)

            else:
                self._log.error("Received unknown message <{}>".format(msg))

                msg.action = 'error'
                msg.args = "Unknown message"
                msg.return_to_sender()

        elif 'greetings' in quest:
            self._log.info("Translated dispatch message as a greeting")    # TODO: Could I add in confidence here ???
            msg.action = 'greet'
            msg.args = "Hello"
            msg.return_to_sender()

        elif 'thanks' in quest:
            self._log.info("Translated dispatch message as a thank you")
            msg.action = 'ack'
            msg.args = "You're welcome"
            msg.return_to_sender()

        elif 'bye' in quest:
            self._log.info("Translated dispatch message as a goodbye. Closing dispatch plugin")
            self._handle_stop(msg, comm, None)
            msg.args.append("Goodbye")
            msg.return_to_sender()

        else:
            self._log.debug("Failed to translate dispatch message: Unknown message")
            msg.action = 'unk'
            msg.args = "I have no idea what you meant"
            msg.return_to_sender()

        # TODO: This is wrong, it prevents us from "forwarding" messages from the cli plugin to the audio plugin
        comm.send(msg, self._log)


    """
    Handle messages as indicated from the nlp results
    """
    async def _handle_stop(self, msg, _comm, _quest):
        self._log.info("Received stop dispatch message. Returning 'stop'")
        msg.action = "send"
        msg.args = "stop"
        msg.return_to_sender()

    async def _handle_music(self, msg, _comm, quest):
        self._log.info("Received request to play music. Determining song to play")

        song = 'Magnet'

        if 'search_query' in quest:
            song = quest['search_query'][0]['value']

        self._log.info("Returning audio request to play {}".format(song))
        msg.action = 'play'
        msg.args = song
        msg.send_to(role='audio')

    async def _handle_find(self, msg, comm, quest):
        self._log.info("Received request to find search query")

        search = Message(plugin=self)
        search.send_to(role='manager')
        search.parent_id = msg.id
        search.action = 'search'

        if 'search_query' in quest:
            search.args = [q['value'] for q in quest['search_query']]
            self._log.info("Searching network for search terms: {}".format(search.args))

        resp = await comm.wait_for_response(search, self._log)

        self._log.info("Returning search results to requesting application")
        msg.response = resp.response
        msg.return_to_sender()
