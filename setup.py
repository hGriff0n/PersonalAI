#!/usr/bin/env python3

# from setuptools import setup, find_packages

# # TODO: I have no idea whether this is accurate or not
# setup(
#     name='PersonalAI',
#     packages=find_packages(),
#     url='https://github.com/hGriff0n/PersonalAI',
#     description='TODO',
#     long_description=open('README.md').read(),
#     install_requires=[
#         "wit>=5.1",
#         "SpeechRecognition>=3.5",
#     ],
#     dependency_links=[
#         'https://github.com/DeepHorizons/tts/tarball/master#egg=package-1.0'
#     ],
#     include_package_data=True,
# )

from os import system

requires = [
    # 'asyncio',
    'wit',
    'PyAudio',
    'SpeechRecognition',
    'pywin32',
    'pydub',
    'git+https://github.com/DeepHorizons/tts'  # This seems to be broken in python 3.6+
]

for package in requires:
    system("pip install {}".format(package))

print("Be sure to install libav or ffmpeg for pydub to work on non-wav files")
