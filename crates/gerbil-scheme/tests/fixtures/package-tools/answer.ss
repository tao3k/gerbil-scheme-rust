(export answer answer-literal main)
(def (answer) 42)
(defrule (answer-literal) 42)
(def (main . _) (void))
