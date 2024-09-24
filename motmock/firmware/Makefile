OBJCOPY:=avr-objcopy
OBJDUMP:=avr-objdump
SIZE:=avr-size

AVRDUDE:=avrdude
AVRDUDE_MCU:=m8
AVRDUDE_PORT:=usb
AVRDUDE_PROG:=avrisp2
AVRDUDE_SPEED:=1

# Fuses: int-osc-8MHz-64ms; BOD-4V
LFUSE:=0x24
HFUSE:=0xD9

AVR_CPU_FREQUENCY_HZ:=8000000

NAME:=motmock
TARGET:=avr-atmega8
RELEASEDIR:=target/$(TARGET)/release
HEX:=$(RELEASEDIR)/$(NAME).hex
ELF:=$(RELEASEDIR)/$(NAME).elf
DASM:=$(RELEASEDIR)/$(NAME).dasm

all: $(HEX) $(DASM)
	$(SIZE) --format=SysV $(ELF)

$(HEX): $(ELF)
	$(OBJCOPY) -R.eeprom -O ihex $(ELF) $(HEX)

$(ELF):
	AVR_CPU_FREQUENCY_HZ=$(AVR_CPU_FREQUENCY_HZ) \
	cargo build --target $(TARGET).json -Z build-std=core --release

.PHONY: $(ELF) # Always run cargo

$(DASM): $(HEX)
	$(OBJDUMP) --disassemble $(ELF) > $(DASM)

dasm: $(DASM)
	less $(DASM)

clean:
	cargo clean

reset:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED) -p $(AVRDUDE_MCU) \
		-U signature:r:/dev/null:i -qq

avrdude:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED) -p $(AVRDUDE_MCU) \
		-t

fuses:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B 10 -p $(AVRDUDE_MCU) \
		-U lfuse:w:$(LFUSE):m -U hfuse:w:$(HFUSE):m

flash: $(HEX)
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED) -p $(AVRDUDE_MCU) \
		-U flash:w:$(HEX)
