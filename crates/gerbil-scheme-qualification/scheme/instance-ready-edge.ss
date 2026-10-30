;; One native Gerbil protocol implementation, shared by the generated variant.
;; The record references the EXISTING frames; it adds no payload/ready queue.
(defstruct mailbox-ready-edge (frames receiver signal issued taken cursor))

(def (mailbox-ready-native-contract)
  (let* ((frames (vector (vector 0 -1 'free #f #f) (vector 1 -1 'free #f #f)
                         (vector 2 -1 'free #f #f)))
         (edge (make-mailbox-ready-edge frames (current-thread) (vector #f) 0 0 0)))
    (mailbox-ready-contract! edge)
    (s64vector 0)))

(def (mailbox-ready-frame edge)
  (let scan ((offset 0))
    (and (< offset 3)
      (let* ((slot (modulo (+ (mailbox-ready-edge-cursor edge) offset) 3))
             (frame (vector-ref (mailbox-ready-edge-frames edge) slot)))
        (if (eq? (vector-ref frame 2) 'ready)
          (begin (mailbox-ready-edge-cursor-set! edge (modulo (1+ slot) 3)) frame)
          (scan (1+ offset)))))))

(def (mailbox-publish-ready! edge id)
  (let ((slot (mailbox-ready-edge-signal edge)) (token (1+ id)))
    ;; A primitive CAS claims the edge across possible green-thread preemption.
    ;; The immutable application id protects against stale/duplicate wake tokens.
    ;; No Scheme pointer crosses FFI, no allocation or handler runs under a lock.
    (when (and (not (##vector-ref slot 0))
               (eq? (##vector-cas! slot 0 token #f) #f))
      (mailbox-ready-edge-issued-set! edge (1+ (mailbox-ready-edge-issued edge)))
      (thread-send (mailbox-ready-edge-receiver edge) token))))

(def (mailbox-take-ready edge)
  (or (mailbox-ready-frame edge)
    (let ((token (thread-receive 5 #f)) (slot (mailbox-ready-edge-signal edge)))
      (unless (and (integer? token) (positive? token)
                   (eqv? (##vector-cas! slot 0 #f token) token))
        (error "native ready wake identity or five-second deadline"))
      ;; Ack BEFORE recheck. A later completion publishes a new wake, even if
      ;; this scan already sees its result. A stale wake returns no capacity.
      (mailbox-ready-edge-taken-set! edge (1+ (mailbox-ready-edge-taken edge)))
      (mailbox-ready-frame edge))))

(def (mailbox-ready-receipt edge)
  (let ((issued (mailbox-ready-edge-issued edge)) (taken (mailbox-ready-edge-taken edge)))
    (unless (<= 0 (- issued taken) 1) (error "native ready wake bound"))
    (s64vector issued taken (- issued taken))))

(def (mailbox-ready-contract! edge)
  ;; Before timing or worker creation, exercise the actual protocol through GC,
  ;; a stale token, duplicate epoch, and a fast slot overtaking ready peers.
  (let* ((frames (mailbox-ready-edge-frames edge)) (frame (vector-ref frames 0))
         (receiver (mailbox-ready-edge-receiver edge)))
    (vector-set! frame 2 'ready)
    (mailbox-publish-ready! edge 0) (mailbox-publish-ready! edge 0) (##gc)
    (unless (and (= (mailbox-ready-edge-issued edge) 1)
                 (eq? frame (mailbox-take-ready edge)) (zero? (mailbox-ready-edge-taken edge)))
      (error "native ready edge contract"))
    (vector-set! frame 2 'free)
    (unless (and (not (mailbox-take-ready edge)) (= (mailbox-ready-edge-taken edge) 1))
      (error "native stale ready wake contract"))
    (thread-send receiver 1)
    (unless (with-catch (lambda (_) #t) (lambda () (mailbox-take-ready edge) #f))
      (error "native duplicate ready wake accepted"))
    (when (thread-receive 0 #f) (error "native ready wake contract residue"))
    (mailbox-ready-edge-cursor-set! edge 0)
    (vector-for-each (lambda (f) (vector-set! f 2 'ready)) frames)
    (unless (eq? (vector-ref frames 0) (mailbox-take-ready edge))
      (error "native ready round robin start"))
    (unless (and (eq? (vector-ref frames 1) (mailbox-take-ready edge))
                 (eq? (vector-ref frames 2) (mailbox-take-ready edge)))
      (error "native ready round robin starvation"))
    (vector-for-each (lambda (f) (vector-set! f 2 'free)) frames)
    (mailbox-ready-edge-issued-set! edge 0)
    (mailbox-ready-edge-taken-set! edge 0)
    (mailbox-ready-edge-cursor-set! edge 0)))
