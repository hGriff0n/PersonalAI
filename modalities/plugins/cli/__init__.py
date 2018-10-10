
import asyncio

from common.msg import Message
from common import plugins

class CliPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super.__init__(self, logger, config=config)

        self._msgs = []
        self._cli_lock = asyncio.Lock()

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

            msg = Message(plugin=self, role='cli')
            msg.action = 'quit'
            msg.send_to(role='manager')
            comm.send(msg)

            return False

        # Send query to network and get response
        msg = Message(plugin=self, role='cli')
        msg.action = 'dispatch'
        msg.args = query
        msg.send_to(role='dispatch')
        resp = await comm.wait_for_response(msg)

        with await self._cli_lock:
            self._msgs.append(msg.args)

        # if resp.action in self._msg_handles:
        #     await self._msg_handles()

        return True

    async def handle_print(self, msg, comm):
        self._log.debug("Received text action")
        with await self._cli_lock:
            self._msgs.append(' '.join(msg.args))

    def _print_all_msgs(self):
        if len(self._msgs) != 0:
            for msg in self._msgs:
                print(msg)

        self._msgs = []
