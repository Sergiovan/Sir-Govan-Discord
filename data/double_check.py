#!/usr/bin/python3

with open("nocontext.txt", 'r') as f:
	lines = f.readlines()
	for x in range(len(lines)-1):
		if lines[x] in lines[x+1:] and x != 848:
			print("'{}' (line {}) was found in {}".format(lines[x].replace('\n', ''), x, [y for y, l in enumerate(lines) if l == lines[x]]))
