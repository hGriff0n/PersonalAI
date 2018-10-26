
import asyncio

from common.msg import Message
from common import plugins

class CliPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super().__init__(logger, config=config)

        self._msgs = []
        self._cli_lock = asyncio.Lock()
        self._role = 'cli'

        self._register_handle('print', CliPlugin.handle_print)
        self._msg_handles = {

        }

    # TODO: The message handling code needs to be put into a separate coroutine
    # Otherwise, we're just single threading the input
    async def run(self, comm):
        with await self._cli_lock:
            self._print_all_msgs()

        query = input("> ")
        if query == "":
            return True

        elif query == "quit":
            self._log.info("Stopping cli plugin")

            msg = Message(plugin=self)
            msg.action = 'quit'
            msg.send_to(role='manager')
            comm.send(msg, self._log)

            return False

        # TODO: Figure out how "quit" conditions would be communicated
        await self._send_query(query, comm)
        # loop = asyncio.get_event_loop()
        # asyncio.run_coroutine_threadsafe(self._send_query(query, comm), loop)

        return True

    async def _send_query(self, query, comm):
        msg = Message(plugin=self)
        msg.action = 'dispatch'
        msg.args = query
        msg.send_to(role='dispatch')
        resp = await comm.wait_for_response(msg, self._log)

        with await self._cli_lock:
            self._msgs.append(resp)

        # if resp.action in self._msg_handles:
        #     await self._msg_handles()


    async def handle_print(self, msg, comm):
        self._log.debug("Received text action")
        with await self._cli_lock:
            self._msgs.append(' '.join(msg.args))

    def _print_all_msgs(self):
        if len(self._msgs) != 0:
            for msg in self._msgs:
                print(msg.json_packet)

        self._msgs = []
