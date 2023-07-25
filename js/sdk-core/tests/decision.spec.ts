import { DecisionState, Answer } from '../src/decision-state';
import { assert } from 'chai';

describe('Test DecisionState', () => {
  it('ask', () => {
    const st = new DecisionState(1, 'alice');
    assert.equal(st.status, 'asked');
  });

  it('setAnswer', () => {
    const st = new DecisionState(1, 'alice');
    st.setAnswer('alice', Uint8Array.of(1), Uint8Array.of(1));
    assert.deepEqual(st.answer, new Answer(Uint8Array.of(1), Uint8Array.of(1)));
    assert.equal(st.status, 'answered');
  });

  it('release', () => {
    const st = new DecisionState(1, 'alice');
    st.setAnswer('alice', Uint8Array.of(1), Uint8Array.of(1));
    st.release();
    assert.equal(st.status, 'releasing');
  });

  it('addSecret', () => {
    const st = new DecisionState(1, 'alice');
    st.setAnswer('alice', Uint8Array.of(1), Uint8Array.of(1));
    st.release();
    st.addSecret('alice', Uint8Array.of(1));
    assert.equal(st.status, 'released');
  });
});
