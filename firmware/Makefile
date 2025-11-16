OBJCOPY:=avr-objcopy
OBJDUMP:=avr-objdump
GDB:=avr-gdb
SIZE:=avr-size
AVR_POSTPROCESS:=avr-postprocess

DWDEBUG:=dwdebug
DWDEBUG_PORT:=ttyUSB0

AVRDUDE:=avrdude
AVRDUDE_MCU:=t861a
AVRDUDE_PORT:=usb
AVRDUDE_PROG:=avrisp2
AVRDUDE_SPEED:=1
AVRDUDE_SPEED_SLOW:=10

# Fuses: ext-high-freq-crystal=4ms; BOD=2.7V; debugWire; WDT-on
LFUSE:=0xEF
HFUSE:=0x8D
EFUSE:=0xFF

AVR_POSTPROCESS_OPT:=\
	-P main-prologue \
	-P bad-interrupt-exit

AVR_CPU_FREQUENCY_HZ:=16000000

NAME:=rpmcontrol
TARGET:=avr-attiny861a
RELEASEDIR:=target/$(TARGET)/release
BIN:=$(RELEASEDIR)/$(NAME).bin
BINPOST:=$(RELEASEDIR)/$(NAME).post.bin
HEX:=$(RELEASEDIR)/$(NAME).hex
HEXPOST:=$(RELEASEDIR)/$(NAME).post.hex
ELF:=$(RELEASEDIR)/$(NAME).elf
DASM:=$(RELEASEDIR)/$(NAME).dasm
DASMPOST:=$(RELEASEDIR)/$(NAME).post.dasm

all: $(BIN) $(BINPOST) $(DASM)
	@-echo
	@-echo "Sizes:"
	@-echo "flash-build:               `du -b $(BIN) | cut -f1`"
	@-echo "flash-post:                `du -b $(BINPOST) | cut -f1`"
	@-$(SIZE) --format=SysV $(ELF) | grep -Ee '^(\.data|\.bss)'

$(BINPOST): $(HEXPOST)
	$(OBJCOPY) -I ihex -O binary $(HEXPOST) $(BINPOST)

$(BIN): $(HEX)
	$(OBJCOPY) -I ihex -O binary $(HEX) $(BIN)

$(HEX): $(ELF)
	$(OBJCOPY) -R.eeprom -O ihex $(ELF) $(HEX)

$(HEXPOST) & $(DASMPOST): $(ELF)
	$(AVR_POSTPROCESS) $(AVR_POSTPROCESS_OPT) -A $(DASMPOST) $(ELF) $(HEXPOST) >/dev/null

$(ELF):
	AVR_CPU_FREQUENCY_HZ=$(AVR_CPU_FREQUENCY_HZ) \
	cargo build --release

.PHONY: $(ELF) # Always run cargo

$(DASM): $(HEX)
	$(OBJDUMP) -j .text --disassemble --disassemble-zeroes $(ELF) > $(DASM)

dasm: $(DASM)
	less $(DASM)

dasm-post: $(DASMPOST)
	less $(DASMPOST)

clean:
	cargo clean

isp:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-t

isp-fuses:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-U lfuse:w:$(LFUSE):m -U hfuse:w:$(HFUSE):m -U efuse:w:$(EFUSE):m

isp-flash: $(HEXPOST)
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED) -p $(AVRDUDE_MCU) \
		-U flash:w:$(HEXPOST)

isp-reset:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-U signature:r:/dev/null:i -q -q

dw:
	$(DWDEBUG) 'device $(DWDEBUG_PORT), te'

dw-flash: $(BINPOST)
	$(DWDEBUG) 'device $(DWDEBUG_PORT), l $(BINPOST), te, qr'

dw-gdbserver:
	$(DWDEBUG) 'device $(DWDEBUG_PORT), reset, te, gdbserver, qr'

dw-reset:
	$(DWDEBUG) 'device $(DWDEBUG_PORT), reset, qr'

gdb:
	$(GDB) -ex 'target remote :4444' $(ELF)
