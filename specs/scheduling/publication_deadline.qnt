// The source can remain open after an individual request's deadline expires.
// Fresh publication must test the request at commit, not only before preparation.
module publication_deadline {
  import continuous_lease as c from "./continuous_lease"
  var live:bool
  var prepared:bool
  var committed:bool
  var acceptedWhileLive:bool
  action init=all {c::init,live'=true,prepared'=false,committed'=false,acceptedWhileLive'=false}
  action idle=all {c::idle,live'=live,prepared'=prepared,committed'=committed,acceptedWhileLive'=acceptedWhileLive}
  action prepare=all {not(committed),c::idle,live'=live,prepared'=true,committed'=committed,acceptedWhileLive'=acceptedWhileLive}
  action expire=all {c::idle,live'=false,prepared'=prepared,committed'=committed,acceptedWhileLive'=acceptedWhileLive}
  action publish=all {prepared,live,not(committed),c::feed(2),live'=live,prepared'=prepared,committed'=true,acceptedWhileLive'=live}
  action unsafeSourceDeadlineOnly=all {prepared,not(committed),c::feed(2),live'=live,prepared'=prepared,committed'=true,acceptedWhileLive'=live}
  val safety=and {c::safety,committed implies acceptedWhileLive}
  run preparationDoesNotExtendPublicationTest=init.then(prepare).then(expire)
    .then(all {assert(not(committed)),assert(c::window==Set()),assert(safety),idle})
  run sourceLifetimeMutationTest=init.then(prepare).then(expire).then(unsafeSourceDeadlineOnly)
    .then(all {assert(not(safety)),idle})
  run expiryAfterCommitDoesNotFreeCustodyTest=init.then(prepare).then(publish).then(expire)
    .then(all {assert(committed),assert(c::window==Set(2)),assert(safety),idle})
  action step=any {prepare,expire,publish,idle}
}
