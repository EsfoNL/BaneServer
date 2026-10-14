#!/bin/env nu

def main [arg: string] {
  print "========[ Hello world! ]================"
  open $arg | from xml | to json | save /tmp/out.json
}
