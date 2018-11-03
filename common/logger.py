#!/usr/bin/env python3

import logging

def create(file, name=None, log_dir=None, fmt=None, level=None):
    hdlr = logging.FileHandler("{}/{}".format(log_dir or '.', file))

    fmt = logging.Formatter(
        fmt or "[%(asctime)s][%(levelname)s][%(filename)s:%(lineno)d] %(message)s"
    )
    hdlr.setFormatter(fmt)

    logger = logging.getLogger(name or __name__)
    logger.addHandler(hdlr)
    logger.setLevel(logging.getLevelName(level or 'DEBUG'))

    return logger
