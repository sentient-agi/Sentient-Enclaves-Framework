all:
	g++ -o test src/main.cc -L../target/debug/ -l:libnsm.so -lssl -lcrypto -ldl -lpthread -lrt
clean:
	rm -rf test
