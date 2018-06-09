

def run(queue):
    while True:
        query = input("> ")
        if query == "quit": break
        queue.put({ 'msg': query })
    queue.put("quit")

# This is really simple to perform actually
    # Benefit: I don't have to declare/import the networking code every app
# Need to add in dispatch function to enable communication to the plugin (queue communicates to the network)
# Issues:
    # Resource contention and back-and-forth communication (handled by dispatch?)
    # I can't exactly spawn plugins at runtime (current architecture focused on one plugin per process)
    # Where would logging statements get sent?
# The only other noticable issue (atm) is in closing the script, but that happens in the "complete" architecture too
