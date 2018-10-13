
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

        self._client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=logger)
        self._register_handle('dispatch', DispatchPlugin.handle_dispatch)

        self._intent_handles = {
            'stop': DispatchPlugin._handle_stop,
            'play_music': DispatchPlugin._handle_music,
            'find': DispatchPlugin._handle_find
        }

    async def run(self, comm):
        await asyncio.sleep(10)
        return True

    async def handle_dispatch(self, msg, comm):
        dispatch = msg.args[0]
        self._log.info("MSG <{}>".format(dispatch))

        # TODO: Can we make the wit.ai call asynchronous?
        query = self._client.message(dispatch)
        quest = query['entities']

        if 'intent' in quest:
            self._log.info("INTENT <{}>".format(quest['intent']))
            intent = quest['intent'][0]

            if intent in self._intent_handles:
                await self._intent_handles[intent](self, msg, comm, quest)

            else:
                self._log.error("Received unknown message <{}>".format(msg))

                msg.action = 'error'
                msg.args = "Unknown message"

        elif 'greetings' in quest:
            self._log.info("GREETING")
            msg.action = 'greet'
            msg.args = "Hello"

        elif 'thanks' in quest:
            self._log.info("GREETING")
            msg.action = 'ack'
            msg.args = "You're welcome"

        elif 'bye' in quest:
            self._log.info("Closing sending plugin")
            self._handle_stop(msg, comm, None)
            msg.args.append("Goodbye")

        else:
            self._log.info("Unknown action")
            msg.action = 'unk'
            msg.args = "I have no idea what you meant"

        msg.return_to_sender()
        comm.send(msg)


    """
    Handle messages as indicated from the nlp results
    """
    async def _handle_stop(self, msg, _comm, _quest):
        msg.action = "send"
        msg.args = "stop"

    async def _handle_music(self, msg, _comm, quest):
        song = 'Magnet'

        if 'search_query' in quest:
            song = quest['search_query'][0]['value']

        self._log.info("PLAYING <{}>".format(song))

        msg.action = 'play'
        msg.args = song
        msg.set_dest(role='audio')

    async def _handle_find(self, msg, comm, quest):
        search = Message(plugin=self, role='dispatch')
        search.send_to(role='search')
        search.parent_id = msg.id
        search.action = 'search'

        if 'search_query' in quest:
            search.args = [q['value'] for q in quest['search_query']]
            self._log.info("FINDING <{}>".format(search.args))

        resp = await comm.wait_for_response(search)
        self._log.info("RESPONSE <{}>".format(resp.response))

        msg.response = resp.response
