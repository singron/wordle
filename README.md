# Wordle Solver

This implements a wordle solver. It uses a minimax technique where it picks a
word to guess that minimizes the maximum possible number of answers.

The first such word is "aesir". This word is not a possible answer. Wordle has
different word lists for the words you can guess and the words that can be
answers.

If you run this program, it will measure its own performance by playing a game
of wordle for every possible answer and measuring statistics. This takes a few
minutes. The current stats are:

    Words=2315 Max=5 Min=2 Avg=3.81 Win=100.00%

You can also supply an answer word as an argument, and it will print out its
sequence of guesses.
