# Example ghidra script showing how to extract the address and
# names of basic blocks. 

# Run this script on the target binary and then feed the 
# output into breakpoints.txt

from ghidra.program.model.block import BasicBlockModel
from ghidra.util.task import TaskMonitor

bbm = BasicBlockModel(currentProgram)
blocks = bbm.getCodeBlocks(TaskMonitor.DUMMY)
block = blocks.next()

while block:
    print "{} {}".format(block.minAddress, block.name)
    print
    block = blocks.next()
