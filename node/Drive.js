const { promisify } = require('util');
const { join: pathJoin } = require('path');
const cbor = require('cbor');
const Document = require('@dashevo/dpp/lib/document/Document');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');

// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  driveOpen,
  driveClose,
  driveCreateInitialStateStructure,
  driveApplyContract,
  driveCreateDocument,
  driveUpdateDocument,
  driveDeleteDocument,
  driveQueryDocuments,
  driveProveDocumentsQuery,
  driveInsertIdentity,
  abciInitChain,
  abciBlockBegin,
  abciBlockEnd,
} = require('neon-load-or-build')({
  dir: pathJoin(__dirname, '..'),
});

const GroveDB = require('./GroveDB');

const appendStack = require('./appendStack');

const decodeProtocolEntity = decodeProtocolEntityFactory();

// Convert the Drive methods from using callbacks to returning promises
const driveCloseAsync = appendStack(promisify(driveClose));
const driveCreateInitialStateStructureAsync = appendStack(
  promisify(driveCreateInitialStateStructure),
);
const driveApplyContractAsync = appendStack(promisify(driveApplyContract));
const driveCreateDocumentAsync = appendStack(promisify(driveCreateDocument));
const driveUpdateDocumentAsync = appendStack(promisify(driveUpdateDocument));
const driveDeleteDocumentAsync = appendStack(promisify(driveDeleteDocument));
const driveQueryDocumentsAsync = appendStack(promisify(driveQueryDocuments));
const driveProveDocumentsQueryAsync = appendStack(promisify(driveProveDocumentsQuery));
const driveInsertIdentityAsync = appendStack(promisify(driveInsertIdentity));
const abciInitChainAsync = appendStack(promisify(abciInitChain));
const abciBlockBeginAsync = appendStack(promisify(abciBlockBegin));
const abciBlockEndAsync = appendStack(promisify(abciBlockEnd));

// Wrapper class for the boxed `Drive` for idiomatic JavaScript usage
class Drive {
  /**
   * @param {string} dbPath
   */
  constructor(dbPath) {
    this.drive = driveOpen(dbPath);
    this.groveDB = new GroveDB(this.drive);
  }

  /**
   * @returns {GroveDB}
   */
  getGroveDB() {
    return this.groveDB;
  }

  /**
   * @returns {Promise<void>}
   */
  async close() {
    return driveCloseAsync.call(this.drive);
  }

  /**
   * @param {External} [transaction=undefined]
   *
   * @returns {Promise<[number, number]>}
   */
  async createInitialStateStructure(transaction = undefined) {
    return driveCreateInitialStateStructureAsync.call(this.drive, undefined);
  }

  /**
   * @param {DataContract} dataContract
   * @param {Date} blockTime
   * @param {External} [transaction=undefined]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async applyContract(dataContract, blockTime, transaction = undefined, dryRun = false) {
    return driveApplyContractAsync.call(
      this.drive,
      dataContract.toBuffer(),
      blockTime,
      !dryRun,
      transaction,
    );
  }

  /**
   * @param {Document} document
   * @param {Date} blockTime
   * @param {External} [transaction=undefined]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async createDocument(document, blockTime, transaction = undefined, dryRun = false) {
    return driveCreateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContract().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      true,
      blockTime,
      !dryRun,
      transaction,
    );
  }

  /**
   * @param {Document} document
   * @param {Date} blockTime
   * @param {External} [transaction=undefined]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async updateDocument(document, blockTime, transaction = undefined, dryRun = false) {
    return driveUpdateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContract().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      blockTime,
      !dryRun,
      transaction,
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} documentId
   * @param {External} [transaction=undefined]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async deleteDocument(
    dataContract,
    documentType,
    documentId,
    transaction = undefined,
    dryRun = false,
  ) {
    return driveDeleteDocumentAsync.call(
      this.drive,
      documentId.toBuffer(),
      dataContract.toBuffer(),
      documentType,
      !dryRun,
      transaction,
    );
  }

  /**
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {External} [transaction=undefined]
   *
   * @returns {Promise<[Document[], number]>}
   */
  async queryDocuments(dataContract, documentType, query = {}, transaction = undefined) {
    const encodedQuery = await cbor.encodeAsync(query);

    const [encodedDocuments, , processingFee] = await driveQueryDocumentsAsync.call(
      this.drive,
      encodedQuery,
      dataContract.id.toBuffer(),
      documentType,
      transaction,
    );

    const documents = encodedDocuments.map((encodedDocument) => {
      const [protocolVersion, rawDocument] = decodeProtocolEntity(encodedDocument);

      rawDocument.$protocolVersion = protocolVersion;

      return new Document(rawDocument, dataContract);
    });

    return [
      documents,
      processingFee,
    ];
  }

  /**
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {External} [transaction=undefined]
   *
   * @returns {Promise<[Document[], number]>}
   */
  async proveDocumentsQuery(dataContract, documentType, query = {}, transaction = undefined) {
    const encodedQuery = await cbor.encodeAsync(query);

    // eslint-disable-next-line no-return-await
    return await driveProveDocumentsQueryAsync.call(
      this.drive,
      encodedQuery,
      dataContract.id.toBuffer(),
      documentType,
      transaction,
    );
  }

  /**
   * @param {Identity} identity
   * @param {External} [transaction=undefined]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async insertIdentity(identity, transaction = undefined, dryRun = false) {
    return driveInsertIdentityAsync.call(
      this.drive,
      identity.toBuffer(),
      !dryRun,
      transaction,
    );
  }

  /**
   * Get the ABCI interface
   * @returns {RSAbci}
   */
  getAbci() {
    const { drive } = this;

    /**
     * @typedef RSAbci
     */
    return {
      /**
       * ABCI init chain
       *
       * @param {InitChainRequest} request
       * @param {External} [transaction=undefined]
       *
       * @returns {Promise<InitChainResponse>}
       */
      async initChain(request, transaction = undefined) {
        const requestBytes = cbor.encode(request);

        const responseBytes = await abciInitChainAsync.call(
          drive,
          requestBytes,
          transaction,
        );

        return cbor.decode(responseBytes);
      },

      /**
       * ABCI init chain
       *
       * @param {BlockBeginRequest} request
       * @param {External} [transaction=undefined]
       *
       * @returns {Promise<BlockBeginResponse>}
       */
      async blockBegin(request, transaction = undefined) {
        const requestBytes = cbor.encode({
          ...request,
          // cborium doesn't eat Buffers
          proposerProTxHash: Array.from(request.proposerProTxHash),
        });

        const responseBytes = await abciBlockBeginAsync.call(
          drive,
          requestBytes,
          transaction,
        );

        return cbor.decode(responseBytes);
      },

      /**
       * ABCI init chain
       *
       * @param {BlockEndRequest} request
       * @param {External} [transaction=undefined]
       *
       * @returns {Promise<BlockEndResponse>}
       */
      async blockEnd(request, transaction = undefined) {
        const requestBytes = cbor.encode(request);

        const responseBytes = await abciBlockEndAsync.call(
          drive,
          requestBytes,
          transaction,
        );

        return cbor.decode(responseBytes);
      },
    };
  }
}

/**
 * @typedef InitChainRequest
 */

/**
 * @typedef InitChainResponse
 */

/**
 * @typedef BlockBeginRequest
 * @property {number} blockHeight
 * @property {number} blockTimeMs - timestamp in milliseconds
 * @property {number} [previousBlockTimeMs] - timestamp in milliseconds
 * @property {Buffer} proposerProTxHash
 */

/**
 * @typedef BlockBeginResponse
 */

/**
 * @typedef BlockEndRequest
 * @property {Fees} fees
 */

/**
 * @typedef Fees
 * @property {number} processingFees
 * @property {number} storageFees
 */

/**
 * @typedef BlockEndResponse
 * @property {number} currentEpochIndex
 * @property {boolean} isEpochChange
 * @property {number} [proposersPaidCount]
 * @property {number} [paidEpochIndex]
 */

module.exports = Drive;
