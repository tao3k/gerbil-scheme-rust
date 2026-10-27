// Existing native owner/root lifecycle with local construction-frame ownership.
// Child GC reachability inside the Scheme foreign frame is a native obligation.
module native_construction {
  const ATOMIC:bool
  const LIMIT:int
  import owner_lifecycle(JOBS=Set(1,2,3),ASSIGNED=Map(1->1,2->2,3->1)) as host from "../owner_lifecycle"
  var stage:str
  var external:int
  var registrations:int
  var charged:int
  var internal:bool
  action keep=all {stage'=stage,external'=external,registrations'=registrations,charged'=charged,internal'=internal}
  action init=all {host::init,stage'="idle",external'=0,registrations'=0,charged'=0,internal'=false}
  action idle=all {host::stutter,keep}
  action reserve=all {host::reserve(1),keep}
  action enter=all {host::admitReserved(1),keep}
  action begin=all {
    host::s.accepted.contains(1),stage=="idle",not(ATOMIC) or LIMIT>=184,host::stutter,
    stage'=if (ATOMIC) "atomic" else "tag",external'=0,registrations'=0,
    charged'=if (ATOMIC) 184 else 0,internal'=ATOMIC
  }
  action allocate(following:str,bytes:int,roots:int):bool=all {
    not(ATOMIC),charged+bytes<=LIMIT,host::stutter,stage'=following,
    external'=roots,registrations'=registrations+1,charged'=charged+bytes,internal'=false
  }
  action tag=all {stage=="tag",allocate("data",16,1)}
  action data=all {stage=="data",allocate("payload",112,2)}
  action payload=all {stage=="payload",allocate("scope",16,3)}
  action leavePayloadScope=all {stage=="scope",host::stutter,stage'="header",external'=1,registrations'=registrations,charged'=charged,internal'=false}
  action header=all {stage=="header",allocate("envelope",24,2)}
  action envelope=all {stage=="envelope",allocate("publish",16,3)}
  action publish=all {stage=="publish",host::stutter,stage'="parent",external'=1,registrations'=registrations,charged'=charged,internal'=false}
  action commit=all {stage=="atomic",host::stutter,stage'="parent",external'=1,registrations'=1,charged'=charged,internal'=false}
  action abort=all {
    Set("tag","data","payload","scope","header","envelope","publish","atomic").contains(stage),host::stutter,
    stage'="failed",external'=0,registrations'=registrations,charged'=charged,internal'=false
  }
  action rejectQuota=all {ATOMIC,LIMIT<184,stage=="idle",host::stutter,stage'="failed",external'=0,registrations'=0,charged'=0,internal'=false}
  action submit=all {stage=="parent",host::submitService(1),stage'="submitted",external'=0,registrations'=registrations,charged'=charged,internal'=false}
  action ready=all {host::readyService(1,false),keep}
  action retire=all {Set("submitted","failed").contains(stage),host::complete(1),stage'="retired",external'=0,registrations'=registrations,charged'=charged,internal'=false}
  action release=all {host::release(1,1),keep}
  action close=all {host::close,keep}
  action unsafeEarlyExport=all {stage=="atomic",host::stutter,stage'=stage,external'=1,registrations'=1,charged'=charged,internal'=true}
  val safety=and {host::safety,0<=external,external<=3,0<=charged,charged<=LIMIT,
    registrations<=if (ATOMIC) 1 else 5,not(internal) or (stage=="atomic" and external==0),
    stage!="parent" or external==1,not(Set("failed","submitted","retired").contains(stage)) or external==0}
  action step=any {reserve,enter,begin,tag,data,payload,leavePayloadScope,header,envelope,publish,commit,abort,rejectQuota,submit,ready,retire,release,close,idle}
}
module composed_construction {
  import native_construction(ATOMIC=false,LIMIT=184).*
  run composedFullConstructionTest=init.then(reserve).then(enter).then(begin).then(tag).then(data).then(payload)
    .then(leavePayloadScope).then(header).then(envelope).then(publish)
    .then(all {assert(external==1),assert(registrations==5),assert(charged==184),assert(safety),idle})
    .then(submit).then(ready).then(retire).then(release).then(all {assert(safety),idle})
}
module atomic_construction {
  import native_construction(ATOMIC=true,LIMIT=184).*
  run atomicFullConstructionTest=init.then(reserve).then(enter).then(begin).then(commit)
    .then(all {assert(external==1),assert(registrations==1),assert(charged==184),assert(safety),idle})
    .then(submit).then(ready).then(retire).then(release).then(all {assert(safety),idle})
  run constructionAbortTest=init.then(reserve).then(enter).then(begin).then(abort).then(close).then(retire).then(release)
    .then(all {assert(external==0),assert(safety),idle})
  run rootEscapeMutationTest=init.then(reserve).then(enter).then(begin).then(unsafeEarlyExport)
    .then(all {assert(not(safety)),idle})
}
module quota_construction {
  import native_construction(ATOMIC=true,LIMIT=183).*
  run quotaRejectsBeforeAllocationTest=init.then(reserve).then(enter).then(rejectQuota).then(retire).then(release)
    .then(all {assert(registrations==0),assert(charged==0),assert(safety),idle})
}
