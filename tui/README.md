# Typing test tui

Monkeytype inspired typing test tui in the terminal made in rust. Words and quotes are from my rust gui typing test repo

### Typing test logic
- Must type the same character as the current letter
- Space goes to next word. If it's the last word, end the test
- Bcakspace deletes a typed character. If it's the start of a word, go to the location of the previous from where you jumped from. E.g. if you typed space in the middle of a word to go to the next word, and backspace, you will be at the letter where you jumped from rather than the end of the previous word.
- WPM formula: wpm = (total chars typed / 5 - error words) / minutes
- accuracy formula: accuracy = (correct characters / total chracters) * 100
