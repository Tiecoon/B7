# UI design 

## Program -> UI

* Brute update
	* triggered when brute has run through an iteration
	* is passed a slice of generators::Input for the run tests
	* should replace wait and instead have this be blocking
* Done
	* signifies that the bruter has finished current generator/queue
	* wait for ui to either issue more commands or to be killed

### Maybe

* updates on completed threads out of number of inputs to run
* a check between thread spawns to see if we need to cancel or pause

#### TODO

* allow bruter to specify/merge multiple input types into one generators::Input

## UI -> Program

* modify current generator(s)
* fetch current generator(s)
* cancel current operation
* fetch cached output

### packet communication

* serialization
	* using serde so we can use whichever format
* communication medium
	* local
		* unix pipes
		* tcp
		* rust channels(maybe)
	* remote
		* tcp
