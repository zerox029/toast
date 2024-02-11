
# TODO

## Initialisation
- [x] Enable interrupts, DMA, and memory space access in the PCI command register
- [x] Memory map BAR 5 register as uncacheable.
- [x] Perform BIOS/OS handoff (if the bit in the extended capabilities is set)
- [x] Reset controller
- [x] Register IRQ handler, using interrupt line given in the PCI register. This interrupt line may be shared with other devices, so the usual implications of this apply.
- [x] Enable AHCI mode and interrupts in global host control register.
- [x] Read capabilities registers. Check 64-bit DMA is supported if you need it.
- [ ] For all the implemented ports:
    - [x] Allocate physical memory for its command list, the received FIS, and its command tables. Make sure the command tables are 128 byte aligned.
    - [x] Memory map these as uncacheable.
    - [ ] Set command list and received FIS address registers (and upper registers, if supported).
    - [ ] Setup command list entries to point to the corresponding command table.
    - [ ] Reset the port.
    - [ ] Start command list processing with the port's command register.
    - [ ] Enable interrupts for the port. The D2H bit will signal completed commands.
    - [ ] Read signature/status of the port to see if it connected to a drive.
    - [ ] Send IDENTIFY ATA command to connected drives. Get their sector size and count.

## Start read/write command

- [ ] Select an available command slot to use.
- [ ] Setup command FIS.
- [ ] Setup PRDT.
- [ ] Setup command list entry.
- [ ] Issue the command, and record separately that you have issued it.

## IRQ handler

- [ ] Check global interrupt status. Write back its value. For all the ports that have a corresponding set bit...
- [ ] Check the port interrupt status. Write back its value. If zero, continue to the next port.
- [ ] If error bit set, reset port/retry commands as necessary.
- [ ] Compare issued commands register to the commands you have recorded as issuing. For any bits where a command was issued but is no longer running, this means that the command has completed.
- [ ] Once done, continue checking if any other devices sharing the IRQ also need servicing.
