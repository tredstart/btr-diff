;;;; btr-diff integration patch for Autolith
;;;; Replaces change-viewer-render with btr-diff --stdin backend.
;;;; Requires: btr-diff binary with --stdin support in PATH.
;;;;
;;;; To apply on another Autolith instance:
;;;;   1. Install btr-diff with --stdin support: cargo install --git <repo-url>
;;;;   2. Have Autolith apply each definition below via self.define, then self.commit.
;;;;
;;;; There are 3 definitions to apply:
;;;;   1. btr-diff--extract-content
;;;;   2. btr-diff--offset-line-number
;;;;   3. btr-diff--parse-modify-line
;;;;   4. change-viewer-render (the main replacement)

;;; 1. Helper: extract content from btr/1 output lines

(defun btr-diff--extract-content (line)
  "Extract the content part after the prefix and line number from a btr line.
For example '+ 4 | content here' returns '4 | content here'."
  (if (> (length line) 2)
      (string-left-trim " " (subseq line 1))
      ""))

;;; 2. Helper: offset line numbers in btr/1 output

(defun btr-diff--offset-line-number (line offset)
  "Offset the line number in a btr/1 line by OFFSET.
LINE is like '+ 4 | content' or '- 3:5'. Returns the content after the prefix
with the line number adjusted."
  (let* ((trimmed (string-left-trim " " (subseq line 1)))
         (space-pos (position #\Space trimmed))
         (pipe-pos (position #\| trimmed)))
    (cond
      (pipe-pos
       (let* ((num-str (string-trim " " (subseq trimmed 0 pipe-pos)))
              (num (ignore-errors (parse-integer num-str))))
         (if num
             (format nil "~D ~A" (+ num offset) (subseq trimmed pipe-pos))
             trimmed)))
      (space-pos
       trimmed)
      (t
       (let ((colon-pos (position #\: trimmed)))
         (if colon-pos
             (let ((n (ignore-errors (parse-integer (subseq trimmed 0 colon-pos))))
                   (m (ignore-errors (parse-integer (subseq trimmed (1+ colon-pos))))))
               (if (and n m)
                   (format nil "~D:~D" (+ n offset) (+ m offset))
                   trimmed))
             (let ((n (ignore-errors (parse-integer trimmed))))
               (if n
                   (format nil "~D" (+ n offset))
                   trimmed))))))))

;;; 3. Helper: parse btr/1 modify lines into styled terminal spans

(defun btr-diff--parse-modify-line (line offset)
  "Parse a btr/1 modify line into terminal spans with red/green change segments.
LINE is like: ~ 3 | version = {\"0.1.0\" \u2192 \"0.2.0\"}
OFFSET is added to the line number. Returns a list of (style . text) cons pairs."
  (let ((pipe-pos (position #\| line)))
    (unless pipe-pos
      (return-from btr-diff--parse-modify-line
        (list (cons ':dim line))))
    (let* ((prefix-raw (subseq line 0 pipe-pos))
           (num-str (string-trim " ~" prefix-raw))
           (num (ignore-errors (parse-integer num-str)))
           (prefix (if num
                       (concatenate 'string "~ " (write-to-string (+ num offset)) " | ")
                       (concatenate 'string prefix-raw "| ")))
           (content (if (< (+ pipe-pos 2) (length line))
                        (subseq line (+ pipe-pos 2))
                        ""))
           (arrow " \u2192 ")
           (arrow-len (length arrow))
           (spans (list (cons ':dim prefix))))
      (let ((pos 0)
            (len (length content)))
        (loop
          (when (>= pos len) (return))
          (let ((brace-start (position #\{ content :start pos)))
            (if (null brace-start)
                (progn
                  (when (< pos len)
                    (push (cons ':plain (subseq content pos)) spans))
                  (return))
                (progn
                  (when (> brace-start pos)
                    (push (cons ':plain (subseq content pos brace-start)) spans))
                  (let ((brace-end (position #\} content :start brace-start)))
                    (if (null brace-end)
                        (progn
                          (push (cons ':plain (subseq content brace-start)) spans)
                          (return))
                        (let* ((inner (subseq content (1+ brace-start) brace-end))
                               (arrow-pos (search arrow inner)))
                          (if arrow-pos
                              (let ((old-text (subseq inner 0 arrow-pos))
                                    (new-text (subseq inner (+ arrow-pos arrow-len))))
                                (push (cons ':failure old-text) spans)
                                (push (cons ':dim arrow) spans)
                                (push (cons ':success new-text) spans))
                              (push (cons ':plain (subseq content brace-start (1+ brace-end))) spans))
                          (setf pos (1+ brace-end))))))))))
      (nreverse spans))))

;;; 4. Main replacement: change-viewer-render using btr-diff --stdin

(defun change-viewer-render
    (&key removed-content added-content
          removed-start-line added-start-line
          source-path syntax-language
          (syntax-highlight-p
            (not (null (or source-path syntax-language))))
          (line-limit *change-viewer-line-limit*))
  "Render a change using btr-diff, falling back to the built-in renderer."
  (declare (ignore syntax-highlight-p syntax-language))
  (handler-case
      (let* ((old-text (or removed-content ""))
             (new-text (or added-content ""))
             (offset (1- (or removed-start-line added-start-line 1)))
             (input (format nil "~A~C~A" old-text #\Nul new-text))
             (btr-output
               (uiop:run-program
                '("btr-diff" "--stdin")
                :input (make-string-input-stream input)
                :output '(:string :stripped t)
                :error-output nil
                :ignore-error-status t)))
        (when (or (null btr-output) (string= btr-output ""))
          (return-from change-viewer-render nil))
        (let ((lines (uiop:split-string btr-output :separator '(#\Newline)))
              (rows nil)
              (count 0))
          (dolist (line lines)
            (when (>= count line-limit) (return))
            (cond
              ((or (string= line "btr/1")
                   (uiop:string-prefix-p "--- " line)
                   (uiop:string-prefix-p "+++ " line))
               nil)
              ((and (plusp (length line)) (char= (char line 0) #\~))
               (push (btr-diff--parse-modify-line line offset) rows)
               (incf count))
              ((and (plusp (length line)) (char= (char line 0) #\+))
               (let ((content (btr-diff--offset-line-number line offset)))
                 (push (list (cons ':success (format nil "+ ~A" content))) rows)
                 (incf count)))
              ((and (plusp (length line)) (char= (char line 0) #\-))
               (let ((content (btr-diff--offset-line-number line offset)))
                 (push (list (cons ':failure (format nil "- ~A" content))) rows)
                 (incf count)))
              ((and (plusp (length line)) (char= (char line 0) #\>))
               (push (list (cons ':dim line)) rows)
               (incf count))
              ((plusp (length line))
               (push (list (cons ':dim line)) rows)
               (incf count))))
          (nreverse rows)))
    (error ()
      (render-diff
       :removed-content        removed-content
       :added-content          added-content
       :removed-start-line     removed-start-line
       :added-start-line       added-start-line
       :source-path            source-path
       :syntax-language        syntax-language
       :line-limit             line-limit
       :sanitize-line-function #'application--sanitize-change-line
       :span-function          #'syntax--terminal-span))))
