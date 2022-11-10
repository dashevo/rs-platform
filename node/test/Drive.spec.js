const fs = require('fs');

const { expect, use } = require('chai');
use(require('dirty-chai'));

const Document = require('@dashevo/dpp/lib/document/Document');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const {
  expectFeeResult,
} = require('./utils');

const Drive = require('../Drive');

const TEST_DATA_PATH = './test_data';

describe('Drive', () => {
  let drive;
  let dataContract;
  let identity;
  let blockInfo;
  let documents;
  let initialRootHash;

  beforeEach(async () => {
    drive = new Drive(TEST_DATA_PATH);

    dataContract = getDataContractFixture();
    identity = getIdentityFixture();

    blockInfo = {
      height: 1,
      epoch: 1,
      timeMs: new Date().getTime(),
    };

    documents = getDocumentsFixture(dataContract);
    initialRootHash = await drive.getGroveDB().getRootHash();
  });

  afterEach(async () => {
    await drive.close();

    fs.rmSync(TEST_DATA_PATH, { recursive: true });
  });

  describe('#createInitialStateStructure', () => {
    it('should create initial tree structure', async () => {
      const result = await drive.createInitialStateStructure();

      // eslint-disable-next-line no-unused-expressions
      expect(result).to.be.undefined;
    });
  });

  describe('#applyContract', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should create contract if not exists', async () => {
      const result = await drive.applyContract(dataContract, blockInfo);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
    });

    it('should update existing contract', async () => {
      await drive.applyContract(dataContract, blockInfo);

      dataContract.setDocumentSchema('newDocumentType', {
        type: 'object',
        properties: {
          newProperty: {
            type: 'string',
          },
        },
        additionalProperties: false,
      });

      const result = await drive.applyContract(dataContract, blockInfo);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
    });

    it('should not create contract with dry run flag', async () => {
      const result = await drive.applyContract(dataContract, blockInfo, false, true);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#createDocument', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.applyContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    context('without indices', () => {
      it('should create a document', async () => {
        const documentWithoutIndices = documents[0];

        const result = await drive.createDocument(documentWithoutIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    context('with indices', () => {
      it('should create a document', async () => {
        const documentWithIndices = documents[3];

        const result = await drive.createDocument(documentWithIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    it('should not create a document with dry run flag', async () => {
      const documentWithoutIndices = documents[0];

      const result = await drive.createDocument(documentWithoutIndices, blockInfo, false, true);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#updateDocument', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.applyContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    context('without indices', () => {
      it('should update a document', async () => {
        // Create a document
        const documentWithoutIndices = documents[0];

        await drive.createDocument(documentWithoutIndices, blockInfo);

        // Update the document
        documentWithoutIndices.set('name', 'Boooooooooooob');

        const result = await drive.updateDocument(documentWithoutIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    context('with indices', () => {
      it('should update the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices, blockInfo);

        // Update the document
        documentWithIndices.set('firstName', 'Bob');

        const result = await drive.updateDocument(documentWithIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    it('should not update a document with dry run flag', async () => {
      const documentWithoutIndices = documents[0];

      // TODO: It should work without document creation
      // await drive.createDocument(documentWithoutIndices, blockInfo);

      documentWithoutIndices.set('name', 'Boooooooooooooooooooooob');

      const result = await drive.updateDocument(documentWithoutIndices, blockInfo, false, true);

      expect(result).to.have.property('processingFee');
      expect(result).to.have.property('storageFee');
      expect(result).to.have.property('removedFromIdentities');

      // TODO: Processing fee doesn't work for v0.23
      expect(result.processingFee).to.be.equals(0);
      expect(result.storageFee).to.be.greaterThan(0, 'storage fee must be higher than 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#deleteDocument', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.applyContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    context('without indices', () => {
      it('should delete the document', async () => {
        // Create a document
        const documentWithoutIndices = documents[3];

        await drive.createDocument(documentWithoutIndices, blockInfo);

        initialRootHash = await drive.getGroveDB().getRootHash();

        const result = await drive.deleteDocument(
          dataContract,
          documentWithoutIndices.getType(),
          documentWithoutIndices.getId(),
          blockInfo,
        );

        expect(result).to.have.property('processingFee');
        expect(result).to.have.property('storageFee');
        expect(result).to.have.property('removedFromIdentities');

        expect(result.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
        expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    context('with indices', () => {
      it('should delete the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices, blockInfo);

        initialRootHash = await drive.getGroveDB().getRootHash();

        const result = await drive.deleteDocument(
          dataContract,
          documentWithIndices.getType(),
          documentWithIndices.getId(),
          blockInfo,
        );

        expect(result).to.have.property('processingFee');
        expect(result).to.have.property('storageFee');
        expect(result).to.have.property('removedFromIdentities');

        expect(result.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
        expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    it('should not delete the document with dry run flag', async () => {
      // Create a document
      const documentWithoutIndices = documents[3];

      await drive.createDocument(documentWithoutIndices, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();

      const result = await drive.deleteDocument(
        dataContract,
        documentWithoutIndices.getType(),
        documentWithoutIndices.getId(),
        blockInfo,
        false,
        true,
      );

      expect(result).to.have.property('processingFee');
      expect(result).to.have.property('storageFee');
      expect(result).to.have.property('removedFromIdentities');

      expect(result.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#queryDocuments', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.applyContract(dataContract, blockInfo);
    });

    it('should query existing documents', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockInfo)),
      );

      // eslint-disable-next-line no-unused-vars
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', blockInfo, {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.deep.equal(documents[4].toObject());

      expect(processingCost).to.be.greaterThan(0);
    });

    it('should query existing documents again', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockInfo)),
      );

      // eslint-disable-next-line no-unused-vars
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', blockInfo, {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.deep.equal(documents[4].toObject());

      expect(processingCost).to.be.greaterThan(0);
    });

    it('should return empty array if documents are not exist', async () => {
      // eslint-disable-next-line no-unused-vars
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', blockInfo, {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(0);
      expect(processingCost).to.be.greaterThan(0);
    });
  });

  describe('#proveDocumentsQuery', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.applyContract(dataContract, blockInfo);
    });

    it('should query existing documents', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockInfo)),
      );

      const result = await drive.proveDocumentsQuery(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(result).to.have.lengthOf(2);

      const [proofs, processingCost] = result;
      expect(proofs).to.be.an.instanceOf(Buffer);
      expect(proofs.length).to.be.greaterThan(0);

      // TODO: We do not calculating processing costs for poofs yet
      expect(processingCost).to.equal(0);
    });
  });

  describe('#insertIdentity', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should create identity if not exists', async () => {
      const result = await drive.insertIdentity(identity, blockInfo);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
    });
  });

  describe('#fetchLatestWithdrawalTransactionIndex', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should return 0 on the first call', async () => {
      const result = await drive.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(0);
    });
  });

  describe('#enqueueWithdrawalTransaction', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should enqueue withdrawal transaction into the queue', async () => {
      await drive.enqueueWithdrawalTransaction(1, Buffer.alloc(32, 1));

      const result = await drive.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(1);
    });
  });

  describe('ABCI', () => {
    describe('InitChain', () => {
      it('should successfully init chain', async () => {
        const request = {};

        const response = await drive.getAbci().initChain(request);

        expect(response).to.be.empty('object');
      });
    });

    describe('BlockBegin', () => {
      beforeEach(async () => {
        await drive.getAbci().initChain({});
      });

      it('should process a block without previous block time', async () => {
        const request = {
          blockHeight: 1,
          blockTimeMs: (new Date()).getTime(),
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        };

        const response = await drive.getAbci().blockBegin(request);

        expect(response.unsignedWithdrawalTransactions).to.be.empty();
        expect(response.epochInfo).to.deep.equal({
          currentEpochIndex: 0,
          isEpochChange: true,
          previousEpochIndex: null,
        });
      });

      it('should process a block with previous block time', async () => {
        const blockTimeMs = (new Date()).getTime();

        await drive.getAbci().blockBegin({
          blockHeight: 1,
          blockTimeMs,
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });

        const response = await drive.getAbci().blockBegin({
          blockHeight: 2,
          blockTimeMs: blockTimeMs + 100,
          proposerProTxHash: Buffer.alloc(32, 1),
          previousBlockTimeMs: blockTimeMs,
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });

        expect(response.unsignedWithdrawalTransactions).to.be.empty();
        expect(response.epochInfo).to.deep.equal({
          currentEpochIndex: 0,
          isEpochChange: false,
          previousEpochIndex: null,
        });
      });
    });

    describe('BlockEnd', () => {
      beforeEach(async () => {
        await drive.getAbci().initChain({});
        await drive.getAbci().blockBegin({
          blockHeight: 1,
          blockTimeMs: (new Date()).getTime(),
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });
      });

      it('should process a block', async () => {
        const request = {
          fees: {
            storageFees: 100,
            processingFees: 100,
          },
        };

        const response = await drive.getAbci().blockEnd(request);

        expect(response).to.have.property('proposersPaidCount');
        expect(response).to.have.property('paidEpochIndex');
      });
    });
  });
});
