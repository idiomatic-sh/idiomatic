Feature: profile
  @succeeds
  Scenario: A profile-ryo command line records both profiles and runs the bench's trace next
    Then profile records the heap and CPU profiles and answers the trace command

  @refuses
  Scenario: A worker built without the heap profiler is refused
    Then profile refuses a worker that writes no heap dump
